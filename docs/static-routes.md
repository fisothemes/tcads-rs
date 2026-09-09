# Connecting without a local AMS router

Installing TwinCAT, either XAE (engineering) or XAR (runtime), installs an AMS router. It runs on
the local machine and listens on TCP port 48898. Every ADS message goes through it, whether the
device on the other end is on the same machine or across the network. Your client talks to the
local router, the local router talks to the target's router, and the routes you add through the
TwinCAT tray icon tell it how. `AdsDevice::connect` opens a socket to `127.0.0.1:48898`, performs a
`PortConnect` handshake, and the router hands back a source address to use.

If your machine cannot run XAE or XAR, there is no local router to talk to. Instead you connect
directly to the router on the target device, using `connect_remote`. There is no handshake and no
address handed back: you choose your own source AMS Net ID, put it in every frame you send, and the
target's router uses it to recognise you. That Net ID has to be registered on the target first, as
a static route.

This is the normal setup for a Linux or macOS development machine, a container, or a Windows box
without TwinCAT.

## 1. Find the target's AMS Net ID

On Windows, right-click the TwinCAT tray icon and open **About TwinCAT System**. On a CX running
Windows CE, open the **Beckhoff CX Configuration Tool** and read **AMS Net Id** on the General tab,
next to the TwinCAT version. On TwinCAT/BSD it is printed on the console banner when you log in.

You need this for the `target` address of `AdsRuntime`, `AdsLogger`, and `AdsSystemService`. You do
not need it for the route entry itself.

## 2. Pick a source AMS Net ID and port

A Net ID is six octets and ends in `.1.1` by convention. It is a logical identifier with no
relation to any IP address, so it does not have to match the address you connect from. It only has
to be unique among the routes on the target.

The convention is to take an IP address you own and append `.1.1`:

```text
192.168.137.1   ->   192.168.137.1.1.1
```

Which IP you take it from is up to you. If your machine has an address on a second adapter, or if
TwinCAT already assigned your machine a Net ID from an earlier install, reusing that is fine even
though it looks nothing like the address the target sees you on.

For the AMS port, pick something above 32768. Beckhoff reserves the lower range for TwinCAT's own
services, and the
[port list](https://infosys.beckhoff.com/content/1033/tc3_grundlagen/116159883.html?id=4295072484284507589)
shows what is already taken.

## 3. Add the route on the target

Open `StaticRoutes.xml` on the target device. Where it lives depends on the version and OS:

| Target                    | Path                                          |
|---------------------------|-----------------------------------------------|
| Windows (TC3 4024.x)      | `C:\TwinCAT\3.1\Target\`                      |
| Windows (TC3 4026.x)      | `C:\ProgramData\Beckhoff\TwinCAT\3.1\Target\` |
| Windows (either)          | `%TWINCATDIR%\3.1\Target\`                    |
| CX with Windows CE (TC3)  | `\Hard Disk\TwinCAT\3.1\Target\`              |
| TwinCAT/RT Linux          | `/etc/TwinCAT/3.1/Target/`                    |
| TwinCAT/BSD               | `/usr/local/etc/TwinCAT/3.1/Target/`          |

The file is a `TcConfig` document with your routes inside a `RemoteConnections` element. Add a
`Route` for your client alongside any that are already there:

```xml
<?xml version="1.0"?>
<TcConfig xmlns:xsi="http://www.w3.org/2001/XMLSchema-instance"
          xsi:noNamespaceSchemaLocation="C:\TwinCAT3\Config\TcConfig.xsd">
  <RemoteConnections>
    <Route>
      <Name>RustClient</Name>
      <Address>192.168.24.4</Address>
      <NetId>192.168.137.1.1.1</NetId>
      <Type>TCP_IP</Type>
      <Flags>0</Flags>
    </Route>
  </RemoteConnections>
</TcConfig>
```

- `Name` is a label. Anything you like, but do not leave it blank.
- `Address` is the IP address the target sees your machine on.
- `NetId` is the source Net ID you picked in step 2. As above, it does not have to resemble
  `Address`; the pair here is a real working route where they share nothing.
- `Type` stays `TCP_IP`.
- `Flags` stays `0`.

Restart the TwinCAT System Service to pick up the change: switch to Config mode, then back to Run
mode.

## 4. Connect

The `source` you pass must carry the Net ID from the route entry. The port is yours to choose.

```rust
use std::time::Duration;
use tcads::client::devices::blocking::AdsDevice;
use tcads::core::AmsAddr;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Net ID must match the <NetId> in the route entry
    let source: AmsAddr = "192.168.137.1.1.1:32777".parse()?;

    // IP and port of the remote AMS router. 48898 is the default.
    let device = AdsDevice::connect_remote(
        "192.168.24.3:48898",
        source,
        Duration::from_secs(5),
    )?;

    println!("Connected. Source address: {}", device.source());

    device.shutdown()?;

    Ok(())
}
```

The subsystem clients take a `target` as well: a Net ID plus the port of the subsystem you want.
Two things work there.

The explicit form is the target's own Net ID, the one you read in step 1:

```rust
use std::time::Duration;
use tcads::client::devices::tokio::AdsRuntime;
use tcads::core::AmsAddr;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let source: AmsAddr = "192.168.137.1.1.1:32777".parse()?;
    let target: AmsAddr = "39.139.122.3.1.1:851".parse()?; // target Net ID + PLC port

    let runtime =
        AdsRuntime::connect_remote("192.168.24.3:48898", source, target, Duration::from_secs(5))
            .await?;

    let count: u32 = runtime.read_value("MAIN.nCount").await?;
    println!("Value: {count}");

    Ok(())
}
```

`AmsAddr::from_local(851)` also works, and saves you looking the Net ID up. It sends
`127.0.0.1.1.1`, which any router resolves to its own machine, so against a remote router it means
"the PLC on the box I am talking to":

```rust
let runtime = AdsRuntime::connect_remote(
    "192.168.24.3:48898",
    source,
    AmsAddr::from_local(851),
    Duration::from_secs(5),
)
.await?;
```

Prefer the explicit Net ID when you care about which device you reached, and when the target router
might forward you on to a third device. `from_local` always stops at the router you connected to.

`AdsLogger::connect_remote` and `AdsSystemService::connect_remote` take a bare `AmsNetId` instead
of a full `AmsAddr`, since their ports are fixed at 100 and 10000. `AmsNetId::local()` is the
equivalent shortcut there.

## Troubleshooting

### The connection opens and then closes immediately. 

The router did not recognise your source
address. Check that the Net ID in `source` matches the route entry exactly, and that you restarted
the System Service after editing the file.

### The connection closes as soon as you send anything 

Remote routers do not implement `PortConnect` and drop the connection if they receive one. Use 
`connect_remote`, not `connect` or `connect_to`, and do not call `port_connect` by hand.

### One route serves one client

The target identifies you by source Net ID alone, so two processes connecting on the same route will 
collide. Give each client its own route entry with its own Net ID. Within a single process this is 
not a problem: `AdsDevice` is `Clone` and every clone shares the one connection, which is what the 
subsystem clients are built on.
