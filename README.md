# cargo-stylus-walnut

This repo is a fork of cargo stylus with walnut debugging capabilities.

## How to build it?

To use this, you need to install `walnut-dbg` tool from [walnut-dbg](https://github.com/walnuthq/walnut-dbg).

Second, install this `cargo` option.

```
$ git clone https://github.com/walnuthq/cargo-stylus-walnut.git
$ cd cargo-stylus-walnut
$ cargo build
```

Install command, so we can use it as `cargo` option:

```
$ cd target/debug && cp walnutdbg cargo-walnutdbg && cp cargo-walnutdbg ~/.cargo/bin && cd -
```

## How to run it?

Lets use https://github.com/OffchainLabs/stylus-hello-world as an example.

In one terminal, start debug node:

```
$ docker run -it --rm --name nitro-dev -p 8547:8547 offchainlabs/nitro-node:v3.5.3-rc.3-653b078 --dev --http.addr 0.0.0.0 --http.api=net,web3,eth,arb,arbdebug,debug
```

In another terminal, compile and deploy the example:

```
$ git clone https://github.com/OffchainLabs/stylus-hello-world
$ cd stylus-hello-world
$ export RPC_URL=http://localhost:8547 && export PRIV_KEY=0xb6b15c8cb491557369f3c7d2c287b053eb229daa9c22138887752191c9520659
$ cargo stylus deploy --private-key=$PRIV_KEY --endpoint=$RPC_URL
...
deployed code at address: 0xa6e41ffd769491a42a6e5ce453259b93983a22ef
deployment tx hash: 0x307b1d712840327349d561dea948d957362d5d807a1dfa87413023159cbb23f2
wasm already activated!

NOTE: We recommend running cargo stylus cache bid da52b25ddb0e3b9cc393b0690ac62245ac772527 0 to cache your activated contract in ArbOS.
Cached contracts benefit from cheaper calls. To read more about the Stylus contract cache, see
https://docs.arbitrum.io/stylus/concepts/stylus-cache-manager
$ export ADDR=0xa6e41ffd769491a42a6e5ce453259b93983a22ef
$ cast send --rpc-url=$RPC_URL --private-key=$PRIV_KEY $ADDR "increment()"
blockHash            0x3f6bea10728836b1f2c37e2aff3b69b1a7175b7607c8dc9df93aa3b4911536ed
blockNumber          5
contractAddress      
cumulativeGasUsed    992585
effectiveGasPrice    100000000
from                 0x3f1Eae7D46d88F08fc2F8ed27FCb2AB183EB2d0E
gasUsed              992585
logs                 []
logsBloom            0x00000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000
root                 
status               1 (success)
transactionHash      0x8d291700d55adce514ada82575c76a5c2657c6d84888af3fe5af4702f2316263
transactionIndex     1
type                 2
blobGasPrice         
blobGasUsed          
authorizationList    
to                   0xA6E41fFD769491a42A6e5Ce453259b93983a22EF
gasUsedForL1         936000
l1BlockNumber        0
timeboosted          false
```

### Run `replay` command

This is the way of using existing `replay` option, that will attach to either `lldb` or `gdb`:

```
$ cargo stylus replay --tx=0x8d291700d55adce514ada82575c76a5c2657c6d84888af3fe5af4702f2316263  --endpoint=$RPC_URL
1 location added to breakpoint 1
warning: This version of LLDB has no plugin for the language "rust". Inspection of frame variables will be limited.
Process 9256 stopped
* thread #1, name = 'main', queue = 'com.apple.main-thread', stop reason = breakpoint 1.1
    frame #0: 0x000000010102cbb8 libstylus_hello_world.dylib`user_entrypoint(len=4) at lib.rs:33:5
   30  	// Define some persistent storage using the Solidity ABI.
   31  	// `Counter` will be the entrypoint.
   32  	sol_storage! {
-> 33  	    #[entrypoint]
   34  	    pub struct Counter {
   35  	        uint256 number;
   36  	    }
Target 0: (cargo-stylus) stopped.
Process 9256 launched: '~/.cargo/bin/cargo-stylus' (arm64)
(lldb) c
Process 9256 resuming
call completed successfully
Process 9256 exited with status = 0 (0x00000000) 
(lldb) q
```

### Run `usertrace` command

We have introduced a new `cargo` option called `usertrace`, that uses similar technology as `replay` option, but it rather attaches to `walnut-dbg`, instead of well known debuggers.

```
$ cargo walnutdbg usertrace  --tx=0x8d291700d55adce514ada82575c76a5c2657c6d84888af3fe5af4702f2316263  --endpoint=$RPC_URL
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 0.36s
(walnut-dbg) target create "~/.cargo/bin/cargo-walnutdbg"
Current executable set to '~/.cargo/bin/cargo-walnutdbg' (arm64).
(walnut-dbg) settings set -- target.run-args  "walnutdbg" "usertrace" "--tx=0x8d291700d55adce514ada82575c76a5c2657c6d84888af3fe5af4702f2316263" "--endpoint=http://localhost:8547" "--child"
(walnut-dbg) b user_entrypoint
Breakpoint 1: no locations (pending).
WARNING:  Unable to resolve breakpoint to any actual locations.
(walnut-dbg) r
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 0.10s
1 location added to breakpoint 1
warning: This version of LLDB has no plugin for the language "rust". Inspection of frame variables will be limited.
Process 9683 launched: '~/.cargo/bin/cargo-walnutdbg' (arm64)
Process 9683 stopped
* thread #1, name = 'main', queue = 'com.apple.main-thread', stop reason = breakpoint 1.1
    frame #0: 0x0000000103088bb8 libstylus_hello_world.dylib`user_entrypoint(len=4) at lib.rs:33:5
   30  	// Define some persistent storage using the Solidity ABI.
   31  	// `Counter` will be the entrypoint.
   32  	sol_storage! {
-> 33  	    #[entrypoint]
   34  	    pub struct Counter {
   35  	        uint256 number;
   36  	    }
(walnut-dbg) calltrace start '^stylus_hello_world::'`
calltrace: Tracing functions matching '^stylus_hello_world::'
Breakpoint ID: 2
Run/continue to collect calls.
(walnut-dbg) c
call completed successfully
Process 9683 resuming
Process 9683 exited with status = 0 (0x00000000)
(walnut-dbg) calltrace stop

--- LLDB Function Trace (JSON) ---
[
  {
    "function": "stylus_hello_world::__stylus_struct_entrypoint::h09ecd85e5c55b994",
    "file": "lib.rs",
    "line": 33,
    "args": [
      { "name": "input", "value": "<unavailable>" },
      { "name": "<anon>", "value": "<unavailable>" }
    ]
  },
  {
    "function": "stylus_hello_world::Counter::increment::h5b9fb276c23de4f4",
    "file": "lib.rs",
    "line": 64,
    "args": [
      { "name": "self", "value": "0x000000016fdeaf48" }
    ]
  },
  {
    "function": "stylus_hello_world::Counter::set_number::h5bd2c4836637ecb9",
    "file": "lib.rs",
    "line": 49,
    "args": [
      { "name": "self", "value": "0x000000016fdeaf48" },
      { "name": "new_number", "value": "<unavailable>" }
    ]
  }
]
----------------------------------
Trace data written to: /tmp/lldb_function_trace.json
(walnut-dbg) q
```

It ends up in:

```
$ cat /tmp/lldb_function_trace.json
[
  {
    "function": "stylus_hello_world::__stylus_struct_entrypoint::h09ecd85e5c55b994",
    "file": "lib.rs",
    "line": 33,
    "args": [
      { "name": "input", "value": "<unavailable>" },
      { "name": "<anon>", "value": "<unavailable>" }
    ]
  },
  {
    "function": "stylus_hello_world::Counter::increment::h5b9fb276c23de4f4",
    "file": "lib.rs",
    "line": 64,
    "args": [
      { "name": "self", "value": "0x000000016fdeaf48" }
    ]
  },
  {
    "function": "stylus_hello_world::Counter::set_number::h5bd2c4836637ecb9",
    "file": "lib.rs",
    "line": 49,
    "args": [
      { "name": "self", "value": "0x000000016fdeaf48" },
      { "name": "new_number", "value": "<unavailable>" }
    ]
  }
]
```

#### Pretty print

Saving the call trace in form of JSON is default, you can use `pretty_trace.py`.
Make sure you have `python` installed:

```
$ python3 -m venv myvenv
$ source ./myvenv/bin/activate
(myvenv) $ pip3 install colorama
(myvenv) $ ../cargo-stylus-walnut/scripts/pretty_trace.py /tmp/lldb_function_trace.json
```

<img width="620" alt="Screenshot 2025-03-28 at 08 00 55" src="https://github.com/user-attachments/assets/1ad1135b-64c6-4bcf-a02e-dd0af9b2e6c6" />
