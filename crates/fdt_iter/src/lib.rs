//! A `#![no_std]` flattened devicetree traversal library
//!
//! ## `Fdt` and `Node`
//!
//! Create an [`Fdt`] from a `&[u8]` and traverse the tree, starting the root:
//!
//! ```no_run
//! use fdt_iter::*;
//!
//! let dtb: &[u8]; // Get `dtb` from file/memory/...
//! # dtb = todo!();
//! let fdt: Fdt = Fdt::from_bytes(dtb).expect("Invalid FDT"); // Or other error handling
//!
//! let root: Node = fdt.root();
//!
//! for compatible in root.compatible() {
//!     println!("Machine compatible with {compatible:?}");
//! }
//! ```
//!
//! ## One-pass subtree traversal
//!
//! A [`Walker`] (created by [`Node::walker()`]) represents a one-pass traversal
//! through the subtree of this node. [`iter()`][Walker::iter] gives the `Iter`
//! corresponding to the subtree root.
//!
//! ```no_run
//! # use fdt_iter::*;
//! # let root: Node = todo!();
//! let walker: Walker = root.walker();
//! let mut root_iter: Iter = walker.iter();
//! ```
//!
//! An `Iter` represents the how far the current node has been traversed. We can
//! iterate through the immediate children of an `Iter` with
//! [`next_child`][Iter::next_child]. Each child given by
//! [`iter.next_child()`][Iter::next_child] is also an `Iter`, which you can use
//! to recursively traverse its subnodes.
//!
//! For example, to find all devices either at the root of a devicetree, or
//! recursively nested in `simple-bus` buses:
//!
//! ```no_run
//! # use fdt_iter::*;
//! # use cstr::cstr;
//! fn find_devices(mut iter: Iter) {
//!     let node = iter.node();
//!     if node.compatible_with("simple-bus").unwrap_or(false) {
//!         while let Some(child) = iter.next_child() {
//!             find_devices(child);
//!         }
//!     } else {
//!         println!("Device {:?}", node.name());
//!         // Maybe find and register the drivers of this device
//!     }
//! }
//!
//! # let fdt: Fdt = todo!();
//! find_devices(fdt.root().walker().iter());
//! ```
//!
//! This will discover the top-level devices `device-foo`, `device-bar`,
//! `device-baz` in this devicetree (various attributes omitted for brevity):
//!
//! ```dts
//! / {
//!     device-foo {};
//!
//!     soc {
//!         compatible = "simple-bus";
//!
//!         device-bar { };
//!         device-baz { };
//!     };
//! };
//! ```
//!
//! Of course, since you can create a [`Walker`] from any [`Node`], the
//! traversal need not start from the root.
//!
//! All [`Iter`] iterators derived from the same [`Walker`] share state so that
//! walking recursively through a tree only requires one pass through the FDT.
//!
//! ## Preorder traversal
//!
//! To encourage recursive traversal using [`Iter`], there's no way to directly
//! get the immediate children of a [`Node`]. However sometimes it's useful to
//! non-recursively enumerate all nodes in a subtree. The
//! [`preorder()`][Node::preorder] functions gives an iterator through all nodes
//! in a subtree, including the subtree root, with parent coming before
//! children:
//!
//! ```no_run
//! # use fdt_iter::*;
//! let root: Node;
//! # root = todo!();
//! for node in root.preorder() {
//!     println!("Node {:?}", node.name());
//! }
//! ```
//!
//! ## Property parsing helpers
//!
//! The [`property`] module has helpers for parsing some property types.

#![warn(unsafe_op_in_unsafe_fn)]
#![cfg_attr(not(test), no_std)]

mod debug_iter;
mod fdt;
mod node;
mod op;
pub mod property;

pub use fdt::*;
pub use node::*;
