# Global variables
PROJECTNAME=unbound-blocklist-generator
# Go related variables.
DISTPATH=dist
GOFILES=$(shell find . -type f -name '*.go' -not -path './vendor/*')

.DEFAULT_GOAL := help

## clean: Clean the projects dist folder
.PHONY: clean
clean:
	@echo " > Cleaning dist folder..."
	@rm -r dist || true

## build: Build the project
.PHONY: build
build: $(DISTPATH)/$(PROJECTNAME)

## build-arm64: Build the project for ARM64
.PHONY: build-arm64
build-arm64: $(DISTPATH)/$(PROJECTNAME).arm64

## install-dependencies: Install all necessary dependencies for this project
.PHONY: install-dependencies
install-dependencies:
	@echo " > Installing missing dependencies to cache..."
	@go mod tidy
	@echo " > Creating vendor cache..."
	@go mod vendor
	@echo " > Done..."

.PHONY: help
help: Makefile
	@echo
	@echo "Choose a command run in "$(PROJECTNAME)":"
	@echo
	@sed -n 's/^##//p' $< | column -t -s ':' |  sed -e 's/^/ /'
	@echo

$(DISTPATH)/$(PROJECTNAME): $(GOFILES) go.mod go.sum
	@echo " > Building binary..."
	@mkdir -p $(DISTPATH)
	@go build -mod=vendor -ldflags '-s' -o ./$(DISTPATH)/$(PROJECTNAME) .
	@echo " > Done... available at $(DISTPATH)/$(PROJECTNAME)"

$(DISTPATH)/$(PROJECTNAME).arm64: $(GOFILES) go.mod go.sum
	@echo " > Building binary..."
	@mkdir -p $(DISTPATH)
	@GOARCH=arm64 go build -mod=vendor -ldflags '-s' -o ./$(DISTPATH)/$(PROJECTNAME).arm64 .
	@echo " > Done... available at $(DISTPATH)/$(PROJECTNAME).arm64"
