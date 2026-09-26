use std::io;

use rustc_hash::FxHashMap;

/// This structure represents a blocklist, which effectively is a trie structure
/// containing blocklists.
/// The trie structure will automatically deduplicate entries in a DNS sensitive
/// way, so, in example: blocking these urls:
/// `[ "api.example.com", "example.com", "sub.example.com" ]`
/// will result in exactly one entry: `"example.com"`, because for unbound, that
/// entry includes all subdomains.
pub enum BlockList {
    Leaf,
    Branch(FxHashMap<Box<str>, BlockList>),
}

impl Default for BlockList {
    fn default() -> Self {
        BlockList::Branch(FxHashMap::default())
    }
}

impl BlockList {
    pub fn new() -> Self {
        Self::default()
    }

    fn make_leaf(&mut self) {
        *self = BlockList::Leaf;
    }

    pub fn is_empty(&self) -> bool {
        match self {
            BlockList::Leaf => true,
            BlockList::Branch(children) => children.is_empty(),
        }
    }

    pub fn add_domain<I, S>(&mut self, parts: I)
    where
        I: DoubleEndedIterator<Item = S>,
        S: Into<Box<str>>,
    {
        let mut node = self;

        for part in parts.rev() {
            match node {
                BlockList::Leaf => return,
                BlockList::Branch(children) => {
                    node = children.entry(part.into()).or_default();
                }
            }
        }

        node.make_leaf();
    }

    pub fn integrate(&mut self, other: BlockList) {
        match (&mut *self, other) {
            (BlockList::Leaf, _) => (),
            (BlockList::Branch(_), BlockList::Leaf) => {
                *self = BlockList::Leaf;
            }
            (BlockList::Branch(self_map), BlockList::Branch(other_map)) => {
                for (label, child) in other_map {
                    self_map.entry(label).or_default().integrate(child);
                }
            }
        }
    }

    pub fn write_to<'a, W>(&'a self, output: &mut W, shared_labels: &mut Vec<&'a [u8]>) -> Result<(), Box<dyn std::error::Error>>
    where
        W: io::Write,
    {
        match self {
            BlockList::Leaf => {
                output.write_all(b"  local-zone: \"")?;

                for label in shared_labels.iter().rev() {
                    output.write_all(label)?;
                    output.write_all(b".")?;
                }

                output.write_all(b"\" always_null\n")?;
                Ok(())
            }
            BlockList::Branch(children) => {
                for (label, child) in children {
                    shared_labels.push(label.as_bytes());
                    child.write_to(output, shared_labels)?;
                    shared_labels.pop();
                }

                Ok(())
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn write_to_buffer(node: &BlockList) -> String {
        let mut out = Vec::new();
        let mut labels = Vec::new();
        node.write_to(&mut out, &mut labels).unwrap();
        String::from_utf8(out).unwrap()
    }

    #[test]
    fn new_node_is_empty() {
        assert!(BlockList::new().is_empty());
    }

    #[test]
    fn add_domain_produces_leaf() {
        let mut node = BlockList::new();

        node.add_domain("ads.example.com".split('.'));

        assert!(!node.is_empty());
        assert_eq!(write_to_buffer(&node), "  local-zone: \"ads.example.com.\" always_null\n");
    }

    #[test]
    fn write_to_emits_each_domain() {
        let mut node = BlockList::new();

        node.add_domain("ads.example.com".split('.'));
        node.add_domain("tracker.example.com".split('.'));

        let output = write_to_buffer(&node);
        let lines: Vec<&str> = output.lines().collect();

        assert_eq!(lines.len(), 2);
        assert!(lines.contains(&"  local-zone: \"ads.example.com.\" always_null"));
        assert!(lines.contains(&"  local-zone: \"tracker.example.com.\" always_null"));
    }

    #[test]
    fn integrate_merges_trees() {
        let mut base = BlockList::new();
        base.add_domain("a.example.com".split('.'));

        let mut other = BlockList::new();
        other.add_domain("b.example.com".split('.'));
        other.add_domain("other.net".split('.'));

        base.integrate(other);

        let output = write_to_buffer(&base);
        let lines: Vec<&str> = output.lines().collect();

        assert_eq!(lines.len(), 3);
        assert!(lines.contains(&"  local-zone: \"a.example.com.\" always_null"));
        assert!(lines.contains(&"  local-zone: \"b.example.com.\" always_null"));
        assert!(lines.contains(&"  local-zone: \"other.net.\" always_null"));
    }

    #[test]
    fn integrate_leaf_collapses_to_leaf() {
        let mut base = BlockList::new();
        base.add_domain("a.example.com".split('.'));

        let mut other = BlockList::new();
        base.add_domain("b.example.com".split('.'));
        other.make_leaf();

        base.integrate(other);

        assert!(matches!(base, BlockList::Leaf));
        assert_eq!(write_to_buffer(&base), "  local-zone: \"\" always_null\n");
    }

    #[test]
    fn add_domain_into_leaf_is_noop() {
        let mut node = BlockList::new();
        node.add_domain("example.com".split('.'));
        node.make_leaf();
        node.add_domain("sub.example.com".split('.'));

        assert!(matches!(node, BlockList::Leaf));
    }
}
