# Unbound Blocklist Generator

A small Go tool that aggregates domain blocklists from multiple upstream sources and generates a configuration file for the [Unbound](https://nlnetlabs.nl/projects/unbound/) DNS resolver. After writing the config, it reloads Unbound automatically.

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

## Configuration

See [exampleconfig.toml](exampleconfig.toml) for a full example.

| Key | Type | Description |
|---|---|---|
| `target_filename` | string | Path to write the generated Unbound config to |
| `blocklist_urls` | string[] | URLs of upstream blocklists to fetch |
| `blocked_domains` | string[] | Additional domains/TLDs to block directly |
| `allowed_domains` | string[] | Domains exempted from blocking |
| `max_parallel_downloads` | int | Max concurrent downloads (0 = number of CPU cores) |

> Note: `allowed_domains` is basically an allow-list that keeps certrain entries and their subdomains from the blocklist, but if a parent-domain is blocked, this won't help either - consider allowing all parent-domains if necessary.

Additioanlly, you can set the `LOG_LEVEL` environment variable to `DEBUG`, `INFO`, `WARN`, or `ERROR` (default: `INFO`) to get more logging if necessary.

## Building

```sh
make install-dependencies # install all dependencies

make build        # build for current arch → dist/unbound-blocklist-generator
make build-arm64  # cross-compile for ARM64
```

Requires Go 1.25+ (probably works with older go-versions as well, but I developed it with 1.25, so any version below is untested).

## Disclaimer

This project is just something I made for my own homeserver. You can use or fork it if you want, but don't expect me to add features for you if I don't need them myself. Use it at your own risk.
