# Unbound Blocklist Generator

A small tool that aggregates domain blocklists from multiple upstream sources and generates a configuration file for the [Unbound](https://nlnetlabs.nl/projects/unbound/) DNS resolver. After writing the config, it reloads Unbound automatically.

## How it works

1. Downloads blocklists from configured URLs in parallel
2. Merges them, deduplicating and simplifying the result list
3. Writes a `server:` config block to the target file
4. Calls `unbound-control reload` to apply the new blocklist

## Usage

Either call

```sh
unbound-blocklist-generator /path/to/config.toml
```

or use the provided systemd files in *systemd/*.

## Dependencies

This implementation uses *libcurl* under the hood, therefore you need to have *curl* installed on your system. As far as I know, all versions >7.5 will do.

Other dependencies are managed via cargo, therefore part of the resulting binary.

## Configuration

See [exampleconfig.toml](exampleconfig.toml) for a full example.

| Key | Type | Description |
|---|---|---|
| `target_filename` | string | Path to write the generated Unbound config to |
| `blocklist_urls` | string[] | URLs of upstream blocklists to fetch |
| `blocked_domains` | string[] | Additional domains/TLDs to block directly |
| `allowed_domains` | string[] | Domains exempted from blocking |
| `max_parallel_downloads` | int | Max concurrent downloads (0 = number of CPU cores) |

> Note: `allowed_domains` is basically an allow-list that keeps certain entries and their subdomains from the blocklist, but if a parent-domain is blocked, this won't help either - consider allowing all parent-domains if necessary.

Additionally, you can set the `LOG_LEVEL` environment variable to `DEBUG`, `INFO`, `WARN`, or `ERROR` (default: `INFO`) to get more logging if necessary.

## Building

```sh
cargo build --release
```

Requires a rust version new enough to compile edition 2024.

## Python Implementation

There is a Python standalone implementation in *unbound-blocklist-generator.py*. That version does not rely on any configuration file or any external library, other than the Python default runtime. The output is the same, but execution is significantly slower. Nevertheless, if you can't use the Go version for whatever reason, feel free to use this Python version.

## Go Implementation

This project was originally implemented in Go before being ported to Rust. You can find the Go source code on the "go-implementation" branch. That version is considered legacy but implements the same features and works just fine.

## Disclaimer

This project is just something I made for my own homeserver. You can use or fork it if you want, but don't expect me to add features for you if I don't need them myself. Use it at your own risk.
