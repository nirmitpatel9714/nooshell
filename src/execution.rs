use std::process::Stdio;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::process::{Child, Command};
use tokio::sync::mpsc;
use crate::config::LanguageConfig;

pub struct ProcessSession {
    pub process: Child,
    pub input_sender: mpsc::Sender<String>,
}

impl ProcessSession {
    pub fn start(
        config: &LanguageConfig,
        output_sender: mpsc::Sender<String>,
    ) -> std::io::Result<Self> {
        let mut cmd = Command::new(&config.cmd);
        cmd.args(&config.args)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());

        let mut process = cmd.spawn()?;

        let mut stdin = process.stdin.take().expect("Failed to open stdin");
        let stdout = process.stdout.take().expect("Failed to open stdout");
        let stderr = process.stderr.take().expect("Failed to open stderr");

        let (input_sender, mut input_receiver) = mpsc::channel::<String>(32);

        // Task to write input to stdin
        tokio::spawn(async move {
            while let Some(mut line) = input_receiver.recv().await {
                if !line.ends_with('\n') {
                    line.push('\n');
                }
                if stdin.write_all(line.as_bytes()).await.is_err() {
                    break;
                }
                let _ = stdin.flush().await;
            }
        });

        let out_sender = output_sender.clone();
        tokio::spawn(async move {
            let mut reader = BufReader::new(stdout).lines();
            while let Ok(Some(line)) = reader.next_line().await {
                if out_sender.send(line).await.is_err() {
                    break;
                }
            }
        });

        let err_sender = output_sender;
        tokio::spawn(async move {
            let mut reader = BufReader::new(stderr).lines();
            while let Ok(Some(line)) = reader.next_line().await {
                if err_sender.send(line).await.is_err() {
                    break;
                }
            }
        });

        Ok(Self {
            process,
            input_sender,
        })
    }

    pub async fn send_input(&self, input: &str) {
        let _ = self.input_sender.send(input.to_string()).await;
    }
}
