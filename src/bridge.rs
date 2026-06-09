use crate::state::SharedState;

pub const STATE_PREFIX: &str = "__NS_SYNC__";

fn escape_for_single_quotes(s: &str) -> String {
    s.replace('\\', "\\\\").replace('\'', "\\'")
}

pub fn injection_code(state: &SharedState, lang: &str) -> Option<String> {
    let json = state.as_json_string();
    if json == "{}" {
        return None;
    }
    let esc = escape_for_single_quotes(&json);
    match lang {
        "py" => Some(format!(
            "import json as _nsj;globals().update(_nsj.loads('{}'))",
            esc
        )),
        "js" => Some(format!(
            "var _nsd=JSON.parse('{}');Object.keys(_nsd).forEach(function(_nsk){{global[_nsk]=_nsd[_nsk]}});",
            esc
        )),
        _ => None,
    }
}

pub fn dump_code(lang: &str) -> Option<String> {
    match lang {
        "py" => Some(
            // Single-line exec() avoids REPL multi-line indentation issues
            "exec(\"import json as _j,builtins as _b\\n_r={}\\nfor _k,_v in globals().copy().items():\\n if not _k.startswith('_') and _k not in dir(_b):\\n  try:\\n   _j.dumps(_v);_r[_k]=_v\\n  except:pass\\nprint('__NS_SYNC__'+_j.dumps(_r,default=str))\\ndel _k,_v,_r,_j,_b\")"
                .to_string(),
        ),
        "js" => Some(
            // Use var, filter to non-underscore non-function, exclude Node internals
            "var _nsr={};Object.keys(global).filter(function(_nsk){\
             return _nsk[0]!=='_'&&typeof global[_nsk]!=='function'\
             &&['_nsr','_nsd','performance','crypto','navigator',\
             'sessionStorage','localStorage'].indexOf(_nsk)<0;\
             }).forEach(function(_nsk){try{JSON.stringify(global[_nsk]);_nsr[_nsk]=global[_nsk]}catch(_nse){}});\
             console.log('__NS_SYNC__'+JSON.stringify(_nsr));\
             delete _nsr;"
                .to_string(),
        ),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::process::{Command, Stdio};

    fn run_python_code(code: &str) -> (String, String) {
        let child = Command::new("python")
            .args(["-c", code])
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .expect("failed to spawn python");
        let output = child.wait_with_output().expect("failed to wait");
        let stdout = String::from_utf8_lossy(&output.stdout).to_string();
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();
        (stdout, stderr)
    }

    #[test]
    fn test_python_injection_roundtrip() {
        let state = SharedState::new();
        state.set("x", serde_json::json!(42));
        state.set("name", serde_json::json!("hello"));

        let inject = injection_code(&state, "py").unwrap();
        let dump = dump_code("py").unwrap();
        eprintln!("inject = {:?}", inject);

        let code = format!("{}\nprint(x)\nprint(name)\n{}", inject, dump);
        let (out, err) = run_python_code(&code);
        eprintln!("stdout = {:?}", out);
        eprintln!("stderr = {:?}", err);

        assert!(out.contains("42"), "output should contain 42, got stdout: {:?} stderr: {:?}", out, err);
        assert!(out.contains("hello"), "output should contain hello, got stdout: {:?} stderr: {:?}", out, err);
        assert!(out.contains("__NS_SYNC__"), "output should contain __NS_SYNC__, got stdout: {:?} stderr: {:?}", out, err);
    }

    #[test]
    fn test_python_dump_captures_variable() {
        let state = SharedState::new();
        let inject = injection_code(&state, "py");
        let dump = dump_code("py").unwrap();

        let code = if let Some(inj) = inject {
            format!("{}\nmyvar = 123\n{}", inj, dump)
        } else {
            format!("myvar = 123\n{}", dump)
        };
        let (out, err) = run_python_code(&code);
        assert!(out.contains("__NS_SYNC__"), "sync prefix missing, stdout: {:?} stderr: {:?}", out, err);
        assert!(out.contains("123"), "variable 123 not in dump: stdout: {:?} stderr: {:?}", out, err);
    }

    #[test]
    fn test_python_js_bridge_commands_valid() {
        // Python injection with state
        let state = SharedState::new();
        state.set("x", serde_json::json!(42));
        let inject_py = injection_code(&state, "py").unwrap();
        let inject_js = injection_code(&state, "js").unwrap();
        eprintln!("inject_py = {:?}", inject_py);
        eprintln!("inject_js = {:?}", inject_js);

        let (out, err) = run_python_code(&inject_py);
        eprintln!("py stdout = {:?} stderr = {:?}", out, err);
        assert!(err.is_empty(), "python exec should not error: {:?}", err);

        // JavaScript: combine inject and use in one -e
        let js_code = format!("{}; console.log(x);", inject_js);
        let out_js = Command::new("node")
            .args(["-e", &js_code])
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .and_then(|c| c.wait_with_output())
            .map(|o| String::from_utf8_lossy(&o.stdout).to_string())
            .unwrap_or_default();
        eprintln!("js stdout = {:?}", out_js);
        assert!(out_js.contains("42"), "js should see x=42: {:?}", out_js);
    }

    #[test]
    fn test_escape_single_quotes() {
        assert_eq!(escape_for_single_quotes("hello"), "hello");
        assert_eq!(escape_for_single_quotes("it's"), "it\\'s");
        assert_eq!(escape_for_single_quotes("a\\b"), "a\\\\b");
        assert_eq!(escape_for_single_quotes("a'\\b"), "a\\'\\\\b");
    }

    #[test]
    fn test_injection_code_empty_state() {
        let state = SharedState::new();
        assert!(injection_code(&state, "py").is_none());
        assert!(injection_code(&state, "js").is_none());
        assert!(injection_code(&state, "rs").is_none());
    }

    #[test]
    fn test_injection_code_py() {
        let state = SharedState::new();
        state.set("x", serde_json::json!(42));
        state.set("name", serde_json::json!("hello"));
        let code = injection_code(&state, "py").unwrap();
        assert!(code.contains("_nsj.loads"), "missing _nsj.loads");
        assert!(code.contains("42"), "missing 42");
        assert!(code.contains("hello"), "missing hello");
        assert!(code.starts_with("import json"), "should start with import json");
    }

    #[test]
    fn test_injection_code_js() {
        let state = SharedState::new();
        state.set("x", serde_json::json!(42));
        let code = injection_code(&state, "js").unwrap();
        assert!(code.contains("JSON.parse"));
        assert!(code.contains("global[_nsk]"));
        assert!(code.contains("42"));
    }

    #[test]
    fn test_injection_code_py_escaping() {
        let state = SharedState::new();
        state.set("msg", serde_json::json!("it's a test"));
        let code = injection_code(&state, "py").unwrap();
        let (out, err) = run_python_code(&code);
        eprintln!("stdout = {:?} stderr = {:?}", out, err);
        assert!(err.is_empty(), "python injection should not error: {:?}", err);
        assert!(code.contains("msg"), "msg key missing");
        assert!(code.contains("it\\'s"), "should escape single quote");
    }

    #[test]
    fn test_injection_code_unknown_lang() {
        let state = SharedState::new();
        state.set("x", serde_json::json!(1));
        assert!(injection_code(&state, "rb").is_none());
    }

    #[test]
    fn test_dump_code_py() {
        let code = dump_code("py").unwrap();
        assert!(code.contains("__NS_SYNC__"));
        assert!(code.contains("_j.dumps"));
        assert!(code.contains("builtins"));
        assert!(code.contains("k.startswith('_')"));
    }

    #[test]
    fn test_dump_code_js() {
        let code = dump_code("js").unwrap();
        assert!(code.contains("__NS_SYNC__"));
        assert!(code.contains("JSON.stringify"));
        assert!(code.contains("global"));
    }

    #[test]
    fn test_dump_code_unknown_lang() {
        assert!(dump_code("rb").is_none());
    }

    #[test]
    fn test_state_prefix_constant() {
        assert_eq!(STATE_PREFIX, "__NS_SYNC__");
    }
}
