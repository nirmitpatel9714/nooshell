# State Bridge

The state bridge enables cross-language variable sharing. Variables defined in one language are available in another language's REPL without manual serialization.

## How it works

### 1. SharedState (`src/state.rs`)

An `Arc<Mutex<serde_json::Map<String, Value>>>` that lives on the `App` and is cloned into every `Pane`. It is the in-memory key-value store shared across all cells and workspaces.

### 2. Injection (`src/bridge.rs`)

Before user code executes, the state is injected into the REPL by sending a code snippet that deserializes the state into the REPL's global scope:

**Python:**
```python
import json as _nsj; globals().update(_nsj.loads('{"x": 42, "name": "world"}'))
```

**JavaScript:**
```javascript
var _nsd=JSON.parse('{"x":42,"name":"world"}');Object.keys(_nsd).forEach(function(_nsk){global[_nsk]=_nsd[_nsk]});
```

### 3. Dump (`src/bridge.rs`)

After user code executes, variables are dumped back from the REPL into `SharedState`:

**Python:**
```python
exec("import json as _j,builtins as _b\n_r={}\nfor _k,_v in globals().copy().items():\n if not _k.startswith('_') and _k not in dir(_b):\n  try:\n   _j.dumps(_v);_r[_k]=_v\n  except:pass\nprint('__NS_SYNC__'+_j.dumps(_r,default=str))\ndel _k,_v,_r,_j,_b")
```

**JavaScript:**
```javascript
var _nsr={};Object.keys(global).filter(function(_nsk){
 return _nsk[0]!=='_'&&typeof global[_nsk]!=='function'
 &&['_nsr','_nsd','performance','crypto','navigator',
 'sessionStorage','localStorage'].indexOf(_nsk)<0;
 }).forEach(function(_nsk){try{JSON.stringify(global[_nsk]);_nsr[_nsk]=global[_nsk]}catch(_nse){}});
 console.log('__NS_SYNC__'+JSON.stringify(_nsr));
 delete _nsr;
```

Output lines prefixed with `__NS_SYNC__` are intercepted by `Pane::poll_output()` and merged into `SharedState` instead of being displayed.

### 4. Protocol

The dump output uses the `__NS_SYNC__` prefix (`state::STATE_PREFIX`). Lines matching this pattern are parsed as JSON and merged into the shared state map. Non-prefixed lines are displayed as normal output.

## Rules for variable sharing

- Only variables that don't start with `_` are shared
- Builtin names (e.g., `print`, `console`) are excluded
- Variables must be JSON-serializable (strings, numbers, booleans, arrays, objects)
- If a variable fails `json.dumps`/`JSON.stringify`, it is silently skipped
- Functions and complex objects are not shared

## Supported languages

| Language | Injection | Dump |
| ---      | :---:     | :--: |
| Python   | Yes       | Yes  |
| JavaScript | Yes     | Yes  |
| Others   | No        | No   |
