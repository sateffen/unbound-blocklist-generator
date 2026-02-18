package main

import (
	"bufio"
	"log/slog"
	"os"
	"os/exec"
	"path/filepath"
	"runtime"
	"strings"
	"sync"
)

func getLogLevel() slog.Level {
	switch strings.ToUpper(os.Getenv("LOG_LEVEL")) {
	case "DEBUG":
		return slog.LevelDebug
	case "WARN":
		return slog.LevelWarn
	case "ERROR":
		return slog.LevelError
	default:
		return slog.LevelInfo
	}
}

func runBlocklistGeneration(conf *Config) error {
	blockListRoot := &BlockListNode{
		isLeaf:   false,
		children: make(map[string]*BlockListNode),
		value:    "",
	}

	var blockListURLWaitGroup sync.WaitGroup
	maxParallelDownloads := max(1, runtime.NumCPU()-1)
	if conf.MaxParallelDownloads > 0 {
		maxParallelDownloads = conf.MaxParallelDownloads
	}
	blockListRequestSemaphore := make(chan bool, maxParallelDownloads)
	blockListURLChannel := make(chan []string, 4096)

	for _, blockListURL := range conf.BlocklistUrls {
		blockListURLWaitGroup.Add(1)
		go parseBlocklistFromURL(blockListURL, conf.AllowedDomains, &blockListURLWaitGroup, blockListRequestSemaphore, blockListURLChannel)
	}

	go func() {
		blockListURLWaitGroup.Wait()
		close(blockListRequestSemaphore)
		close(blockListURLChannel)
	}()

	slog.Info("Adding globally blocked TLDs...")
	for _, domain := range conf.BlockedDomains {
		parsedUrl, err := parseURLToParts(domain, conf.AllowedDomains)
		if err != nil {
			slog.Warn("could not add blocked domain", slog.String("domain", domain), slog.Any("error", err))
			continue
		}

		blockListRoot.addDomain(parsedUrl)
	}

	for urlToAdd := range blockListURLChannel {
		blockListRoot.addDomain(urlToAdd)
	}

	targetFile, err := os.Create(conf.TargetFilename)
	if err != nil {
		slog.Error("could not write to output file, error", slog.Any("error", err))
		return err
	}
	// we close it later as well, but is more a "safetynet", just to make sure the file gets closed at all
	defer targetFile.Close()

	targetFileWriter := bufio.NewWriterSize(targetFile, 1024*1024)
	targetFileWriter.WriteString("server:\n")
	blockListRoot.writeToWriter("", targetFileWriter)

	// before we can call unbound-control, we must flush and close the file, else unbound can't read it
	if err = targetFileWriter.Flush(); err != nil {
		slog.Error("error while writing output to target file", slog.Any("error", err))
		return err
	}
	if err = targetFile.Close(); err != nil {
		slog.Error("error closing target file", slog.Any("error", err))
		return err
	}
	slog.Info("successfully wrote blocklist", slog.String("targetfile", conf.TargetFilename))
	return nil
}

func notifyUnbound() error {
	err := exec.Command("unbound-control", "reload").Run()
	if err != nil {
		slog.Error("error reloading unbound", slog.Any("error", err))
		return err
	}
	slog.Info("successfully reloaded unbound")
	return nil
}

func main() {
	globalLogger := slog.New(slog.NewTextHandler(os.Stdout, &slog.HandlerOptions{
		Level: getLogLevel(),
	}))
	slog.SetDefault(globalLogger)

	configFilePath, err := filepath.Abs(os.Args[1])
	if err != nil {
		slog.Error("could not normalize config file path", slog.Any("error", err))
		os.Exit(1)
	}

	slog.Info("loading config", slog.String("configFilePath", configFilePath))

	conf, err := LoadConfig(configFilePath)
	if err != nil {
		slog.Error("could not load config", slog.Any("error", err))
		os.Exit(1)
	}

	if err = runBlocklistGeneration(conf); err != nil {
		os.Exit(1)
	}

	if err = notifyUnbound(); err != nil {
		os.Exit(1)
	}

	slog.Info("Done!")
}
