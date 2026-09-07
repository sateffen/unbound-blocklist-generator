package main

import "bufio"

type BlockListNode struct {
	// NOTE:
	// To safe memory, these nodes don't know their own value, only their parents do.
	// That way we don't have to store the values of nodes twice, but only once. That
	// way we can safe some memory, without losing any information.
	children map[string]*BlockListNode
}

func (bln *BlockListNode) addDomain(urlParts []string) {
	it := bln
	for i := len(urlParts) - 1; i >= 0 && !it.isLeaf(); i-- {
		it = it.addChild(urlParts[i])
	}

	it.makeLeaf()
}

func (bln *BlockListNode) addChild(childValue string) *BlockListNode {
	if bln.isLeaf() {
		return bln
	}

	existingChild, ok := bln.children[childValue]
	if ok {
		return existingChild
	}

	newChild := &BlockListNode{
		children: make(map[string]*BlockListNode),
	}

	bln.children[childValue] = newChild

	return newChild
}

func (bln *BlockListNode) writeToWriter(writer *bufio.Writer, value string, suffix string) {
	if bln.isLeaf() {
		writer.WriteString("  local-zone: \"")
		writer.WriteString(value)
		if suffix != "" {
			writer.WriteByte('.')
			writer.WriteString(suffix)
		}
		writer.WriteString(".\" always_null\n")
		return
	}

	childSuffix := value
	if suffix != "" {
		childSuffix = value + "." + suffix
	}

	for key, child := range bln.children {
		child.writeToWriter(writer, key, childSuffix)
	}
}

func (bln *BlockListNode) makeLeaf() {
	bln.children = nil
}

func (bln *BlockListNode) isLeaf() bool {
	return bln.children == nil
}
