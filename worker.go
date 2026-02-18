package main

import (
	"bufio"
	"errors"
	"log/slog"
	"net/http"
	"strings"
	"sync"
)

func parseURLToParts(urlString string, allowedDomains []string) ([]string, error) {
	strippedUrl := strings.TrimSpace(urlString)

	if len(strippedUrl) == 0 {
		return nil, errors.New("urlstring is empty")
	}

	if strippedUrl[0] == '#' || strippedUrl[0] == '/' || strippedUrl[0] == '!' {
		return nil, errors.New("urlstring is a comment, no url")
	}

	for _, allowedDomain := range allowedDomains {
		if strings.HasSuffix(strippedUrl, allowedDomain) {
			return nil, errors.New("urlstring is allowed by global allowlist")
		}
	}

	if len(strippedUrl) > 3 && strippedUrl[0] == '|' && strippedUrl[1] == '|' && strippedUrl[len(strippedUrl)-1] == '^' {
		strippedUrl = strippedUrl[2 : len(strippedUrl)-1]
	}

	if len(strippedUrl) > 2 && strippedUrl[0] == '*' && strippedUrl[1] == '.' {
		strippedUrl = strippedUrl[2:]
	}

	splitParts := strings.Split(strippedUrl, ".")

	for _, part := range splitParts {
		if len(part) == 0 {
			return nil, errors.New("empty part in domain")
		}

		for _, c := range part {
			if !isValidDomainChar(c) {
				return nil, errors.New("invalid character in domain")
			}
		}
	}

	return splitParts, nil
}

func isValidDomainChar(c rune) bool {
	return (c >= 'a' && c <= 'z') ||
		(c >= 'A' && c <= 'Z') ||
		(c >= '0' && c <= '9') ||
		c == '-' || c == '_'
}

func parseBlocklistFromURL(blocklistURL string, allowsDomains []string, wg *sync.WaitGroup, semaphore chan bool, urlChannel chan<- []string) {
	defer wg.Done()

	semaphore <- true
	defer func() {
		<-semaphore
	}()

	slog.Info("downloading blocklist", slog.String("blocklist", blocklistURL))
	response, err := http.Get(blocklistURL)
	if err != nil {
		slog.Error("error while downloading", slog.String("blocklist", blocklistURL), slog.Any("error", err))
		return
	}
	if response.StatusCode != 200 {
		slog.Error("failed statuscode as response", slog.String("blocklist", blocklistURL), slog.Int("statuscode", response.StatusCode))
		return
	}

	s := bufio.NewScanner(response.Body)
	defer response.Body.Close()
	for s.Scan() {
		urlParts, err := parseURLToParts(s.Text(), allowsDomains)
		if err == nil {
			urlChannel <- urlParts
		}
	}
	if err := s.Err(); err != nil {
		slog.Error("error readong blocklist", slog.String("blocklist", blocklistURL), slog.Any("error", err))
	}
}
