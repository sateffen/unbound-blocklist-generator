package main

import (
	"log/slog"

	"github.com/BurntSushi/toml"
)

type Config struct {
	TargetFilename       string   `toml:"target_filename"`
	AllowedDomains       []string `toml:"allowed_domains"`
	BlockedDomains       []string `toml:"blocked_domains"`
	BlocklistUrls        []string `toml:"blocklist_urls"`
	MaxParallelDownloads int      `toml:"max_parallel_downloads"`
}

// LoadConfig loads the file from given path and parses it as toml file, decoding it
// to an usable Config.
func LoadConfig(filePath string) (*Config, error) {
	var conf Config

	metaData, err := toml.DecodeFile(filePath, &conf)
	if err != nil {
		return nil, err
	}

	undecodedKeys := metaData.Undecoded()
	if len(undecodedKeys) > 0 {
		slog.Warn(
			"found unknown keys in config",
			slog.String("configFilePath", filePath),
			slog.Any("undecodedKeys", undecodedKeys),
		)
	}

	return &conf, nil
}
