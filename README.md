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
curl localhost:8080/geoip/<someipaddress>
```

You can signup and get the geoiplite city license for free from [maxmind.com](maxmind.com).

## Healthcheck:
```
curl localhost:8080/health-check
```

## Usage:

```
geoip [OPTIONS] --license <license>

FLAGS:
    -h, --help       Prints help information
    -V, --version    Prints version information

OPTIONS:
    -a, --address <address>    ip address to bind to [default: 0.0.0.0]
    -l, --license <license>    maxmind license (its free, but you have to sign up)
    -t, --logtype <logtype>    sets the logging type [json/terminal] [default: terminal]
    -p, --port <port>          http service port [default: 8080]
```
