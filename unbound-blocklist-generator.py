#!/usr/bin/env python3

"""
This is an alternative implementation of the unbound-blocklist-generator, containing everything necessary
for generating those blocklists and notifying unbound in one file.
Compared to the Go implementation, this version is a bit simpler to keep it readable and graspable, but the
output is the same. Everything necessary is implemented in this one file, including configuration (so no
external configuration file, see configuration area below the imports). Additionally, it doesn't rely on
any external library, but only the python runtime.
"""

from __future__ import annotations
import gzip
from io import TextIOWrapper
from typing import Generator, List
from urllib.request import urlopen, Request
from urllib.error import HTTPError, URLError
import re
import subprocess
import sys

#######################
# START Configuration #
#######################

# TARGET_FILENAME = "/etc/unbound/conf.d/unbound-blocklist.generated.conf"
TARGET_FILENAME = "./unbound-blocklist.generated.conf"

ALLOWED_DOMAINS = {
    "dns.mullvad.net",
}

BLOCKED_DOMAINS = {
	"su",
	"ru",
	"tk",
	"box",
	"zip",
	"mov",
	"top",
}

BLOCKLIST_URLS = {
    "https://raw.githubusercontent.com/sjhgvr/oisd/refs/heads/main/domainswild2_big.txt",
	"https://raw.githubusercontent.com/hagezi/dns-blocklists/refs/heads/main/wildcard/pro.txt",
	"https://raw.githubusercontent.com/hagezi/dns-blocklists/refs/heads/main/wildcard/doh.txt",
	"https://dbl.ipfire.org/lists/ads/domains.txt",
	"https://dbl.ipfire.org/lists/doh/domains.txt",
	"https://dbl.ipfire.org/lists/phishing/domains.txt",
	"https://dbl.ipfire.org/lists/malware/domains.txt",
}

#####################
# END Configuration #
#####################

class BlockListNode:
    def __init__(self, value):
        self.value = value
        self.is_leaf = False
        self.children = {}

    def add_domain(self, url_parts: List[str]) -> None:
        it = self
        for i in range(len(url_parts) - 1, -1, -1):
            if it.is_leaf:
                break
            it = it.add_child(url_parts[i])
        it.make_leaf()

    def add_child(self, child_value: str) -> BlockListNode:
        if self.is_leaf:
            return self

        if child_value in self.children:
            return self.children[child_value]

        new_child = BlockListNode(child_value)
        self.children[child_value] = new_child
        return new_child

    def make_leaf(self) -> None:
        self.is_leaf = True
        self.children.clear()

    def write_to_file(self, target_file: TextIOWrapper, suffix="") -> None:
        if self.is_leaf:
            # Write the complete domain with FQDN trailing dot
            target_file.write(f'  local-zone: "{self.value}')
            if suffix:
                target_file.write(f'.{suffix}')
            target_file.write('." always_null\n')
            return

        child_suffix = self.value
        if suffix:
            child_suffix = f"{self.value}.{suffix}"

        for child in self.children.values():
            child.write_to_file(target_file, child_suffix)

_VALID_DOMAIN = re.compile(r'^[a-zA-Z0-9_-]+(?:\.[a-zA-Z0-9_-]+)*$')
def is_valid_domain(domain: str) -> bool:
    """
    Checks whether a given domain is a valid domain that we can insert into our tree.

    Returns True if the domain is valid for entering into the blocklist.
    """
    if not domain:
        return False

    # Check allowlist
    for allowed_domain in ALLOWED_DOMAINS:
        if domain.endswith(allowed_domain):
            # here we return false, because we don't want to add it to the blocklist
            return False

    return bool(_VALID_DOMAIN.match(domain))

def parse_domain_from_line(line: str) -> str | None:
    """
    Parse a domain from a blocklist line.

    Handles various formats:
    - Plain domains: example.com
    - Adblock format: ||example.com^
    - Wildcard format: *.example.com
    - Comments: # comment, / comment, ! comment

    Returns the domain string or None if line should be skipped.
    """
    stripped = line.strip()

    # Skip empty lines
    if not stripped:
        return None

    # Skip comments
    if stripped[0] in '#/!':
        return None

    # Handle adblock format like ||example.com^
    if len(stripped) > 3 and stripped.startswith("||") and stripped.endswith("^"):
        stripped = stripped[2:-1]

    # Handle wildcards like *.example.com
    if len(stripped) > 2 and stripped.startswith("*."):
        stripped = stripped[2:]

    return stripped


def load_domains_from_url(url) -> Generator[str, None, None]:
    try:
        print(f"Downloading {url}...")
        req = Request(url, headers={"Accept-Encoding": "gzip"})
        with urlopen(req, timeout=30) as response:
            content_encoding = response.headers.get("Content-Encoding", "")
            raw = gzip.GzipFile(fileobj=response) if content_encoding == "gzip" else response
            for line in TextIOWrapper(raw, encoding="utf-8"):
                domain = parse_domain_from_line(line)
                if domain:
                    yield domain

    except HTTPError as e:
        print(f"HTTP Error for {url}: {e.code} - {e.reason}")
    except URLError as e:
        print(f"URL Error for {url}: {e.reason}")
    except Exception as e:
        print(f"Unexpected error for {url}: {e}")


def main() -> int:
    print("Starting unbound blocklist generation...")

    # Initialize blocklist root node
    blocklist_root = BlockListNode("")

    print("Adding globally blocked TLDs...")
    for domain in BLOCKED_DOMAINS:
        if is_valid_domain(domain):
            domain_parts = domain.split(".")
            blocklist_root.add_domain(domain_parts)

    # Download and process each blocklist URL
    for url in BLOCKLIST_URLS:
        for domain in load_domains_from_url(url):
            if is_valid_domain(domain):
                domain_parts = domain.split(".")
                blocklist_root.add_domain(domain_parts)

    print(f"Writing output to {TARGET_FILENAME}...")
    try:
        with open(TARGET_FILENAME, "w", encoding="utf-8", buffering=1024*1024) as f:
            f.write("server:\n")
            blocklist_root.write_to_file(f, "")
        print(f"Successfully wrote blocklist to {TARGET_FILENAME}")
    except IOError as e:
        print(f"Error writing to output target file: {e}")
        return 1

    # Reload unbound
    print("Reloading unbound...")
    try:
        result = subprocess.run(
            ["unbound-control", "reload"],
            capture_output=True,
            text=True,
            timeout=10
        )
        if result.returncode != 0:
            print(f"Warning: unbound-control returned {result.returncode}")
            if result.stderr:
                print(f"Error output: {result.stderr}")
        else:
            print("Successfully reloaded unbound")
    except FileNotFoundError:
        print("Warning: unbound-control not found (is unbound installed?)")
    except subprocess.TimeoutExpired:
        print("Warning: unbound-control timed out")
    except Exception as e:
        print(f"Error reloading unbound: {e}")

    print("Done!")
    return 0


if __name__ == '__main__':
    sys.exit(main())
