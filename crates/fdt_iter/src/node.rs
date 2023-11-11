use crate::debug_iter::IteratorDebug;
use crate::fdt::*;
use crate::property::*;
use core::ffi::CStr;
use core::fmt::Debug;

/// State of a one-pass subtree traversal
///
/// See [crate level documentation][crate] for an introduction.
///
/// A [`Walker`] does not have much use on its own, but since the current
/// position in the FDT is stored in the [`Walker`] rather than individual
/// [`Iter`] iterators, a [`Walker`] must be kept for the duration of the
/// traversal.
///
/// A stack of [`Iter`] iterators can all be derived from a [`Walker<'a>`], and
/// have lifetime parameters of `Iter<'a, 'b>`:
///
/// ```text
/// - Walker<'a>
///   - Iter<'a, 'a>
///     - Iter<'a, 'b1>
///       - Iter<'a, 'b2>
///         ...
/// ```
///
#[derive(Clone)]
pub struct Walker<'a> {
    iter: OpIter<'a>,
    depth: usize,
}

impl<'a> Walker<'a> {
    fn depth(&self) -> usize {
        self.depth
    }

    fn underlying(&self) -> OpIter<'a> {
        self.iter.clone()
    }

    /// Get the [`Iter`] of the subtree root
    pub fn iter(&'a mut self) -> Iter<'a, 'a> {
        Iter {
            node: Node(self.underlying()),
            depth: self.depth(),
            walker: self,
        }
    }

    fn next(&mut self) -> Option<Op<'a>> {
        let res = self.iter.next();
        match res {
            Some(Op::BeginNode { .. }) => self.depth += 1,
            Some(Op::EndNode) => self.depth -= 1,
            _ => {}
        }
        res
    }

    fn preorder(self) -> impl Iterator<Item = Node<'a>> {
        PreorderIter(self)
    }
}

/// A devicetree node
#[derive(Clone)]
pub struct Node<'a>(pub(crate) OpIter<'a>);

impl Node<'_> {
    /// Create a one pass traversal [`Walker`]
    ///
    /// In the most common case where the tree is traversed recursively, the
    /// recursive function can take a parameter of type [`Iter`] and the initial
    /// caller can pass it `node.walker().iter()`, keeping the intermediate `Walker`
    /// alive outside:
    ///
    /// ```no_run
    /// # use fdt_iter::*;
    /// fn recursive_traversal(mut iter: Iter) { /* ... */ }
    ///
    /// let node: Node;
    /// # node = todo!();
    /// recursive_traversal(node.walker().iter());
    /// ```
    pub fn walker(&self) -> Walker {
        Walker {
            iter: self.0.clone(),
            depth: 0,
        }
    }

    /// Get all nodes of this subtree in preorder
    pub fn preorder(&self) -> impl Iterator<Item = Node> {
        self.walker().preorder()
    }

    /// Byte offset of this node from flattened devicetre
    pub fn offset(&self) -> usize {
        self.0.offset()
    }

    /// The node's name, including the unit name
    pub fn name(&self) -> &CStr {
        let Some(Op::BeginNode { name }) = self.0.peek() else {
            panic!("Bad beginning of FDT node")
        };

        name
    }

    /// All properties of the node
    pub fn properties(&self) -> impl Iterator<Item = (&CStr, &[u8])> {
        let mut iter = self.0.clone();
        assert!(matches!(iter.next(), Some(Op::BeginNode { .. })));
        PropertiesIter(iter)
    }

    /// Get a property by its name
    pub fn property(&self, name: &str) -> Option<&[u8]> {
        self.properties().find_map(|(prop_name, value)| {
            (name.as_bytes() == prop_name.to_bytes()).then_some(value)
        })
    }

    /// `compatible` strings of this node
    pub fn compatible(&self) -> Option<impl Iterator<Item = &CStr> + Clone + Debug> {
        let compatible = self.property("compatible").unwrap_or(b"");
        string_list(compatible).map(|iter| iter.debug())
    }

    /// Check if node is compatible with a compatible string
    pub fn compatible_with(&self, compatible: &str) -> Option<bool> {
        self.compatible()
            .map(|mut iter| iter.any(|c| c.to_bytes() == compatible.as_bytes()))
    }

    /// Get the `phandle` of a node
    pub fn phandle(&self) -> Option<u32> {
        self.property("phandle").and_then(u32)
    }

    /// Get the `status` property of a node
    ///
    /// Defaults to [`Status::Okay`] (i.e. `okay`)
    pub fn status(&self) -> Option<Status<'_>> {
        self.property("status")
            .map_or(Some(Status::Okay), Status::from_bytes)
    }

    /// Get the `#{name}-cells` property of a node
    pub fn cells(&self, name: &str) -> Option<u32> {
        self.properties()
            .find_map(|(prop_name, value)| {
                let want = b"#"
                    .iter()
                    .chain(name.as_bytes().iter())
                    .chain(b"-cells".iter());
                prop_name.to_bytes().iter().eq(want).then_some(value)
            })
            .and_then(u32)
    }

    /// Get the `#address-cells` property of a node.
    ///
    /// Defaults to `2`.
    pub fn address_cells(&self) -> usize {
        self.cells("address").unwrap_or(2) as usize
    }

    /// Get the `#size-cells` property of a node
    ///
    /// Defaults to `1`.
    pub fn size_cells(&self) -> usize {
        self.cells("size").unwrap_or(2) as usize
    }

    /// Get the `reg` property as `(addr, size)` pairs
    pub fn reg(
        &self,
        address_cells: usize,
        size_cells: usize,
    ) -> Option<impl Iterator<Item = (u64, u64)> + Clone + Debug + '_> {
        reg_list(
            self.property("reg").unwrap_or(b""),
            address_cells,
            size_cells,
        )
    }
}

struct PropertiesIter<'a>(OpIter<'a>);

impl<'a> Iterator for PropertiesIter<'a> {
    type Item = (&'a CStr, &'a [u8]);

    fn next(&mut self) -> Option<Self::Item> {
        match self.0.next() {
            Some(Op::Prop { name, value }) => Some((name, value)),
            Some(_) => None,
            None => panic!("Unexpected FDT end"),
        }
    }
}

struct PreorderIter<'a>(Walker<'a>);

impl<'a> Iterator for PreorderIter<'a> {
    type Item = Node<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            let iter = self.0.underlying().clone();
            let op = self.0.next().expect("Unexpected FDT end");
            if self.0.depth() == 0 {
                break None;
            }
            if let Op::BeginNode { .. } = op {
                break Some(Node(iter));
            }
        }
    }
}

/// A node being traversed as part of a one-pass subtree traversal
///
/// See [crate level documentation][crate] for an introduction.
///
/// An [`Iter`] allows for efficient one-pass traversal of a subtree, but is
/// derived from a [`Walker`] and cannot be cloned. To reuse the node for later,
/// call [`node()`][Iter::node()] and save the resulting [`Node`].
///
/// An `Iter<'a, 'b>` is derived from a `Walker<'a>`.
pub struct Iter<'a, 'b> {
    walker: &'b mut Walker<'a>,
    node: Node<'a>,
    depth: usize,
}

impl<'a, 'b> Iter<'a, 'b> {
    /// Get the [`Node`] being traversed by this [`Iter`]
    pub fn node(&self) -> Node<'a> {
        self.node.clone()
    }

    /// Get the [`Iter`] of the next immediate child
    ///
    /// Use this pattern to get child [`Iter`] iterators of an [`Iter`]:
    ///
    /// ```no_run
    /// # use fdt_iter::*;
    /// let iter: Iter;
    /// # iter = todo!();
    /// while let Some(child) = iter.next_child() {
    ///     // child: Iter
    /// }
    /// ```
    ///
    /// Due to Rust type system limitations, [`Iter`] cannot implement [`Iterator`]
    /// and thus cannot use the `for ... in` syntax.
    pub fn next_child<'c>(&'c mut self) -> Option<Iter<'a, 'c>> {
        while self.walker.depth() != self.depth + 1 {
            self.walker.next();
        }

        loop {
            let node = Node(self.walker.underlying());
            let op = self.walker.next().expect("Unexpected FDT end");
            return match op {
                Op::BeginNode { .. } => Some(Iter {
                    node,
                    depth: self.walker.depth() - 1,
                    walker: self.walker,
                }),
                Op::EndNode => None,
                Op::Prop { .. } => continue,
            };
        }
    }
}
