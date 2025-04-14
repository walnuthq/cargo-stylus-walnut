# cargo-stylus-walnut

This repo is a fork of cargo stylus with walnut debugging capabilities.

## How to build it?

To use this, you need to install `walnut-dbg` tool from [walnut-dbg](https://github.com/walnuthq/walnut-dbg).

Second, install this `cargo` option.

```bash
git clone https://github.com/walnuthq/cargo-stylus-walnut.git
cd cargo-stylus-walnut
cargo build
```

Install command, so we can use it as `cargo` option:

```bash
cargo install --path main
```

## How to run it?

Lets use https://github.com/OffchainLabs/stylus-hello-world as an example.

In one terminal, start debug node:

```bash
docker run -it --rm --name nitro-dev -p 8547:8547 offchainlabs/nitro-node:v3.5.3-rc.3-653b078 --dev --http.addr 0.0.0.0 --http.api=net,web3,eth,arb,arbdebug,debug
```

In another terminal, compile and deploy the example:

```bash
git clone https://github.com/OffchainLabs/stylus-hello-world
cd stylus-hello-world
export RPC_URL=http://localhost:8547
export PRIV_KEY=0xb6b15c8cb491557369f3c7d2c287b053eb229daa9c22138887752191c9520659
cargo stylus deploy --private-key=$PRIV_KEY --endpoint=$RPC_URL
```

and you can expect the output like this
```text
...
deployed code at address: 0xda52b25ddb0e3b9cc393b0690ac62245ac772527
deployment tx hash: 0x307b1d712840327349d561dea948d957362d5d807a1dfa87413023159cbb23f2
wasm already activated!

NOTE: We recommend running cargo stylus cache bid da52b25ddb0e3b9cc393b0690ac62245ac772527 0 to cache your activated contract in ArbOS.
Cached contracts benefit from cheaper calls. To read more about the Stylus contract cache, see
https://docs.arbitrum.io/stylus/concepts/stylus-cache-manager
$ export ADDR=0xda52b25ddb0e3b9cc393b0690ac62245ac772527
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
transactionHash      0x88b0ad9daa0b701d868a5f9a0132db7c0402178ba44ed8dec4ba76784c7194fd
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

```bash
cargo stylus replay \
  --tx=0x88b0ad9daa0b701d868a5f9a0132db7c0402178ba44ed8dec4ba76784c7194fd \
  --endpoint=$RPC_URL
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

First, make sure you installed `colorama` package:

```bash
$ python3 -m venv myvenv
$ source ./myvenv/bin/activate
(myvenv) $ pip3 install colorama
```

We have introduced a new `cargo` option called `usertrace`, that uses similar technology as `replay` option, but it rather attaches to `walnut-dbg`, instead of well known debuggers.

``` bash
$ cargo walnutdbg usertrace \
  --tx=0x88b0ad9daa0b701d868a5f9a0132db7c0402178ba44ed8dec4ba76784c7194fd \
  --endpoint=$RPC_URL
=== WALNUT FUNCTION CALL TREE ===
└─ #1 stylus_hello_world::__stylus_struct_entrypoint::h09ecd85e5c55b994 (lib.rs:33)
    input = size=4
    <anon> = stylus_sdk::host::VM { 0=<unavailable> }
  └─ #2 stylus_hello_world::Counter::increment::h5b9fb276c23de4f4 (lib.rs:64)
      self = 0x000000016fdeaa78
    └─ #3 stylus_hello_world::Counter::set_number::h5bd2c4836637ecb9 (lib.rs:49)
        self = 0x000000016fdeaa78
        new_number = ruint::Uint<256, 4> { limbs=unsigned long[4] { [0]=1, [1]=0, [2]=0, [3]=0 } }
```

In your terminal, it will look as:

<img width="699" alt="Screenshot 2025-04-14 at 13 09 47" src="https://github.com/user-attachments/assets/45ea3aaa-afa7-48fe-a832-7bf878903a6b" />

You may see the calltrace in form of JSON in:

```
/tmp/lldb_function_trace.json
```

By default, it does not follow functions from `stylus_sdk::`, if you want to see those, use `--verbose-usertrace` option, e.g.:

```bash
$ cargo walnutdbg usertrace \
  --tx=0x88b0ad9daa0b701d868a5f9a0132db7c0402178ba44ed8dec4ba76784c7194fd \
  --endpoint=$RPC_URL --verbose-usertrace
```

Or, if you want to track calls from other libraries, just use `--trace-external-usertrace` as follows:

```bash
cargo walnutdbg usertrace \
  --tx=0x88b0ad9daa0b701d868a5f9a0132db7c0402178ba44ed8dec4ba76784c7194fd \
  --endpoint=$RPC_URL --verbose-usertrace --trace-external-usertrace="std,core,other_contract"
```

and it will track calls from `std::`, `core` and `other_contract::`.
