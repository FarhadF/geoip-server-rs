# geoip-server-rs
Blazing fast geoip server written in Rust


## Build:
1. Install rust
2. Clone repo and build optimized for production
```
git clone https://FarhadF/geoip-server-rs
cd geoip-server-rs
cargo build --release 
```
The binary will be located at ```./target/release/geoip-server-rs```
## Run:
```
./target/release/geoip-server-rs -l <maxmind license token>
```
```
curl localhost:8080/geoip/json/<someipaddress>
```

You can signup and get the geoiplite city license for free from [maxmind.com](maxmind.com).

## Healthcheck:
```
curl localhost:8080/health-check
```

## Usage:

```
Usage: geoip-server-rs --license <Value> --address <Value> --port <Value>

Options:
  -l, --license <Value>  sets the maxmind license key
  -a, --address <Value>  sets the server address
  -p, --port <Value>     sets the server port
  -h, --help             Print help
  -V, --version          Print version
```
