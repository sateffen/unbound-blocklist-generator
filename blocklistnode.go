package main

import "bufio"

type BlockListNode struct {
	isLeaf   bool
	children map[string]*BlockListNode
	value    string
}

func (bln *BlockListNode) addDomain(urlParts []string) {
	it := bln
	for i := len(urlParts) - 1; i >= 0 && !it.isLeaf; i-- {
		it = it.addChild(urlParts[i])
	}

	it.makeLeaf()
}

func (bln *BlockListNode) addChild(childValue string) *BlockListNode {
	if bln.isLeaf {
		return bln
	}

	existingChild, ok := bln.children[childValue]
	if ok {
		return existingChild
	}

	newChild := &BlockListNode{
		isLeaf:   false,
		children: make(map[string]*BlockListNode),
		value:    childValue,
	}

	bln.children[childValue] = newChild

	return newChild
}

func (bln *BlockListNode) writeToWriter(suffix string, writer *bufio.Writer) {
	if bln.isLeaf {
		writer.WriteString("  local-zone: \"")
		writer.WriteString(bln.value)
		if suffix != "" {
			writer.WriteByte('.')
			writer.WriteString(suffix)
		}
		writer.WriteByte('.')
		writer.WriteString("\" always_null\n")
		return
	}

	childSuffix := bln.value
	if suffix != "" {
		childSuffix = bln.value + "." + suffix
	}

	for _, child := range bln.children {
		child.writeToWriter(childSuffix, writer)
	}
}

func (bln *BlockListNode) makeLeaf() {
	if !bln.isLeaf {
		bln.isLeaf = true
		bln.children = nil
	}
}
