#![allow(clippy::too_long_first_doc_paragraph)]

use core::ffi::CStr;

#[derive(Debug)]
pub enum ParseError {
    InvalidHeader,
    InvalidMagic,
    BufferTooSmall,
    InvalidToken,
}

type Result<T> = core::result::Result<T, ParseError>;

/// Extract u32 from bytes
/// (The DTB is always initialized memory — the MaybeUninit
/// dance and its assume_init_ref unsafe are gone tree-wide here.)
fn bytes_to_u32(bytes: &[u8]) -> Option<u32> {
    let init_bytes = bytes.get(..4)?;
    Some(u32::from_be_bytes(init_bytes.try_into().unwrap()))
}

/// Extract u32 from bytes + offset
fn bytes_to_u32_offset(bytes: &[u8], offset: usize) -> Option<u32> {
    let init_bytes = bytes.get(offset..offset + 4)?;
    Some(u32::from_be_bytes(init_bytes.try_into().unwrap()))
}

/// Extract u64 from bytes
fn bytes_to_u64(bytes: &[u8]) -> Option<u64> {
    let init_bytes = bytes.get(..8)?;
    Some(u64::from_be_bytes(init_bytes.try_into().unwrap()))
}

/// Extract u32 from bytes, but cast as u64
fn bytes_to_u32_as_u64(bytes: &[u8]) -> Option<u64> {
    let init_bytes = bytes.get(..4)?;
    Some(u32::from_be_bytes(init_bytes.try_into().unwrap()).into())
}

fn align4(n: usize) -> usize {
    n + (0usize.wrapping_sub(n) & 3)
}

/// DeviceTree is the class entrypoint to the Devicetree operations.
/// This code focuses only on parsing a Flattened Devicetree without using the heap.
/// The Devicetree specification can be found here:
/// https://www.devicetree.org/specifications/
#[derive(Debug)]
pub struct DeviceTree<'a> {
    data: &'a [u8], // Reference to the underlying data in memory
    header: FdtHeader, // Parsed structure of the header
}

/// The FDT header's fixed size (the two-pass raw-DTB construction's
/// pass-1 prefix).
pub const FDT_HEADER_SIZE: usize = size_of::<FdtHeader>();

impl<'a> DeviceTree<'a> {
    /// Create new DeviceTree based on memory pointed to by data.
    /// Result is error if the header can't be parsed correctly.
    pub fn new(data: &'a [u8]) -> Result<Self> {
        FdtHeader::new(data, false).map(|header| Self { data, header })
    }

    pub fn size(&self) -> usize {
        self.header.totalsize as usize
    }

    /// Given a pointer to the dtb as a usize, return a DeviceTree struct.
    /// from_usize is REMOVED (fdt is now unsafe-free — the
    /// #[forbid(unsafe_code)] regression lock); the two-pass raw-pointer
    /// dance lives at the boot callers (aarch64 main9/vminit, riscv64's
    /// main) as header_totalsize + new over caller-formed slices.
    /// Pass 1 of that two-pass construction: parse just the header prefix
    /// to learn the DTB's totalsize.  `prefix` must hold at least
    /// FDT_HEADER_SIZE bytes.
    pub fn header_totalsize(prefix: &[u8]) -> Result<usize> {
        let h = FdtHeader::new(prefix, true)?;
        Ok(h.totalsize as usize)
    }

    /// Return slice containing `structs` area in FDT
    fn structs(&self) -> &[u8] {
        let start = self.header.off_dt_struct as usize;
        let size: usize = self.header.size_dt_struct as usize;
        &self.data[start..(start + size)]
    }

    /// Return slice containing `strings` area in FDT (all null terminated)
    fn strings(&self) -> &'a [u8] {
        let start = self.header.off_dt_strings as usize;
        let size: usize = self.header.size_dt_strings as usize;
        &self.data[start..(start + size)]
    }

    pub fn root(&self) -> Option<Node> {
        self.node_from_index(0, 0)
    }

    pub fn children<'b>(&'b self, parent: &'b Node) -> impl Iterator<Item = Node> + 'b {
        // Start searching linearly after node.start (which points to the start of the parent)
        let mut i = parent.next_token_start;
        let child_depth = parent.depth + 1;

        core::iter::from_fn(move || {
            let child = self.node_from_index(i, child_depth)?;
            if parent.encloses(&child) {
                i = child.start + child.total_len;
                Some(child)
            } else {
                None
            }
        })
    }

    /// Find the parent of child.
    pub fn parent(&self, child: &Node) -> Option<Node> {
        // Search from the root of the tree down using the depth and the bounds of the nodes
        // to find the parent.
        fn find_parent(dt: &DeviceTree, node: Node, child: &Node) -> Option<Node> {
            if !node.encloses(child) || node.depth >= child.depth {
                return None;
            }
            // At this point, we enclose the child, and we're higher up
            if node.depth + 1 < child.depth {
                // Descend into children
                for c in dt.children(&node) {
                    if let Some(found_parent) = find_parent(dt, c, child) {
                        return Some(found_parent);
                    }
                }
            }
            // Must be the parent
            Some(node)
        }

        let root = self.root();
        root.and_then(|n| find_parent(self, n, child))
    }

    pub fn node_name(&self, node: &Node) -> Option<&str> {
        Self::inline_str(self.structs(), node.name_start)
    }

    pub fn property(&self, node: &Node, prop_name: &str) -> Option<Property> {
        self.properties(node).find(|p| self.property_name(p) == Some(prop_name))
    }

    pub fn property_name(&self, prop: &Property) -> Option<&str> {
        Self::inline_str(self.strings(), prop.name_start)
    }

    pub fn property_value_bytes(&self, prop: &Property) -> Option<&[u8]> {
        let value_end = prop.value_start + prop.value_len;
        self.structs().get(prop.value_start..value_end)
    }

    pub fn property_value_as_u32(&self, prop: &Property) -> Option<u32> {
        let value_end = prop.value_start + prop.value_len;
        self.structs().get(prop.value_start..value_end).and_then(bytes_to_u32)
    }

    /// Extract the property value as a u64 — e.g. /chosen's
    /// linux,initrd-start/end, carried as 8-byte big-endian cells in the
    /// host-patched DTB.
    pub fn property_value_as_u64(&self, prop: &Property) -> Option<u64> {
        self.property_value_bytes(prop).and_then(bytes_to_u64)
    }

    pub fn property_value_as_u32_iter(&self, prop: &Property) -> impl Iterator<Item = u32> + '_ {
        let mut value_i = prop.value_start;
        let value_end = prop.value_start + prop.value_len;
        core::iter::from_fn(move || {
            if value_i >= value_end {
                return None;
            }
            let (start, end) = (value_i, value_i + 4);
            value_i = end;
            self.structs().get(start..end).and_then(bytes_to_u32)
        })
    }

    /// Return the node's #address-cells and #size-cells values as a tuple
    fn node_address_size_cells(&self, node: Option<Node>) -> (usize, usize) {
        let address_cells = node
            .and_then(|n| self.property(&n, "#address-cells"))
            .and_then(|p| self.property_value_as_u32(&p))
            .unwrap_or(2) as usize;
        let size_cells = node
            .and_then(|n| self.property(&n, "#size-cells"))
            .and_then(|p| self.property_value_as_u32(&p))
            .unwrap_or(1) as usize;
        (address_cells, size_cells)
    }

    fn consume_cells(&self, value_i: usize, num_cells: usize) -> Option<u64> {
        let bytes_fn = if num_cells == 1 { bytes_to_u32_as_u64 } else { bytes_to_u64 };
        let start = value_i;
        let end = value_i + (num_cells * 4);
        self.structs().get(start..end).and_then(bytes_fn)
    }

    /// Return the reg values as u64 whether the size is 1 or 2 cells.
    /// Doesn't support > 2 cells.
    pub fn property_reg_iter(&self, node: Node) -> impl Iterator<Item = RegBlock> + '_ {
        // Get the address-cells and size-cells from the parent
        let parent = self.parent(&node);
        let (address_cells, size_cells) = self.node_address_size_cells(parent);

        // If reg doesn't exist, start and len will be zero and None will be returned from the iter
        let prop = self.property(&node, "reg");
        let (value_start, value_len) = prop.map_or((0, 0), |p| (p.value_start, p.value_len));
        let mut value_i = value_start;
        let value_end = value_start + value_len;

        core::iter::from_fn(move || {
            // size_cells may be 0 for reg (implies no len)
            if address_cells == 0 || address_cells > 2 || size_cells > 2 {
                return None;
            }

            let address_size = address_cells * 4;
            let len_size = size_cells * 4;

            // End if not enough bytes to parse address and len
            let remaining = value_end - value_i;
            if (address_size + len_size) > remaining {
                return None;
            }

            let addr = self.consume_cells(value_i, address_cells)?;
            value_i += address_size;

            let len = if size_cells > 0 { self.consume_cells(value_i, size_cells) } else { None };
            value_i += len_size;

            Some(RegBlock { addr, len })
        })
    }

    /// Return the ranges values as u64 whether the size is 1 or 2 cells.
    /// Doesn't support > 2 cells.
    pub fn property_range_iter(&self, node: Node) -> impl Iterator<Item = Range> + '_ {
        // Get the address-cells and size-cells from the parent
        let parent = self.parent(&node);
        let (parent_address_cells, _) = self.node_address_size_cells(parent);
        let (address_cells, size_cells) = self.node_address_size_cells(Some(node));

        // If ranges doesn't exist, start and len will be zero and None will be returned from the iter
        let prop = self.property(&node, "ranges");
        let (value_start, value_len) = prop.map_or((0, 0), |p| (p.value_start, p.value_len));
        let mut value_i = value_start;
        let value_end = value_start + value_len;

        // If the length is zero, handle the identity range as a special case
        let is_identity = value_i == value_end;
        let mut identity_returned = false;

        core::iter::from_fn(move || {
            if is_identity {
                if !identity_returned {
                    identity_returned = true;
                    return Some(Range::Identity);
                }
                return None;
            }

            // size_cells must not be 0 for ranges
            if address_cells == 0 || size_cells == 0 || address_cells > 2 || size_cells > 2 {
                return None;
            }
            if parent_address_cells == 0 || parent_address_cells > 2 {
                return None;
            }

            let address_size = address_cells * 4;
            let parent_address_size = parent_address_cells * 4;
            let len_size = size_cells * 4;

            // End if not enough bytes to parse 2x address and len
            let remaining = value_end - value_i;
            if (address_size + parent_address_size + len_size) > remaining {
                return None;
            }

            let child_bus_addr = self.consume_cells(value_i, address_cells)?;
            value_i += address_size;

            let parent_bus_addr = self.consume_cells(value_i, parent_address_cells)?;
            value_i += parent_address_size;

            let len = self.consume_cells(value_i, size_cells)?;
            value_i += len_size;

            Some(Range::Translated(RangeMapping { child_bus_addr, parent_bus_addr, len }))
        })
    }

    /// Get the reg values, translated by ranges of the parent
    pub fn property_translated_reg_iter(
        &self,
        node: Node,
    ) -> impl Iterator<Item = TranslatedReg> + '_ {
        let mut reg_iter = self.property_reg_iter(node);
        let mut curr_reg = reg_iter.next();

        // Work on each reg element in turn
        core::iter::from_fn(move || {
            if let Some(reg) = curr_reg {
                curr_reg = reg_iter.next();

                // Walk from child to parents, translating by ranges at each step
                let mut translated_reg = reg;
                let mut curr_parent = self.parent(&node);
                while curr_parent.is_some() {
                    if let Some(parent) = curr_parent {
                        if parent.is_root() {
                            return Some(TranslatedReg::Translated(translated_reg));
                        }

                        // Find a range containing the regblock
                        let mut translated = false;
                        for range in self.property_range_iter(parent) {
                            if let Some(new_reg) = range.translate(translated_reg) {
                                translated_reg = new_reg;
                                translated = true;
                                break;
                            }
                        }

                        if !translated {
                            return Some(TranslatedReg::Unreachable);
                        }

                        curr_parent = self.parent(&parent);
                    }
                }
            }
            None
        })
    }

    /// Return the first node carrying `phandle = <phandle>`, or None when
    /// no node does.  Interrupt-map parents are phandle references —
    /// resolving them is how the tuple stride's parent half is known.
    pub fn find_phandle(&self, phandle: u32) -> Option<Node> {
        self.nodes().find(|n| {
            self.property(n, "phandle").and_then(|p| self.property_value_as_u32(&p))
                == Some(phandle)
        })
    }

    /// The node's `#interrupt-cells` value, or None when the property is
    /// absent.  Absence is never a silent default: a node participating
    /// in interrupt routing without the property is malformed.
    pub fn node_interrupt_cells(&self, node: &Node) -> Option<usize> {
        self.property(node, "#interrupt-cells")
            .and_then(|p| self.property_value_as_u32(&p))
            .map(|v| v as usize)
    }

    /// The byte slice + cell-count layout for `node`'s interrupt-map, or
    /// None when any required piece is absent or malformed: the property
    /// itself, the node's own #address-cells/#interrupt-cells, the first
    /// tuple's parent phandle resolving to a node, and that parent's
    /// #address-cells/#interrupt-cells (at most 3 — PciIrqMap carries
    /// three specifier cells).  Every absence is a loud None: no default
    /// cell count is ever assumed.
    fn interrupt_map_layout(&self, node: &Node) -> Option<(&[u8], usize, usize, usize, usize)> {
        let prop = self.property(node, "interrupt-map")?;
        let value = self.property_value_bytes(&prop)?;
        let child_addr_cells = self
            .property(node, "#address-cells")
            .and_then(|p| self.property_value_as_u32(&p))? as usize;
        let child_intr_cells = self.node_interrupt_cells(node)?;
        // The parent half of the stride comes from the FIRST tuple's
        // phandle (all 16 committed entries share the GIC parent).
        let phandle_off = (child_addr_cells + child_intr_cells) * 4;
        let parent_phandle = value.get(phandle_off..).and_then(bytes_to_u32)?;
        let parent = self.find_phandle(parent_phandle)?;
        let parent_addr_cells = self
            .property(&parent, "#address-cells")
            .and_then(|p| self.property_value_as_u32(&p))? as usize;
        let parent_intr_cells = self.node_interrupt_cells(&parent)?;
        if parent_intr_cells > 3 {
            return None;
        }
        Some((value, child_addr_cells, child_intr_cells, parent_addr_cells, parent_intr_cells))
    }

    /// Iterate a pci host node's `interrupt-map` property as decoded
    /// tuples.  The stride derives from the pci node's cell counts plus
    /// the interrupt parent's (resolved via the first tuple's phandle);
    /// an absent map, absent cell counts, or an unresolvable parent
    /// yields an empty iterator (the lookup layer's loud None).
    pub fn interrupt_map_iter(&self, node: Node) -> impl Iterator<Item = PciIrqMap> + '_ {
        let (value, ca, ci, pa, pi) =
            self.interrupt_map_layout(&node).map_or((&[][..], 0, 0, 0, 0), |l| l);
        decode_interrupt_map(value, ca, ci, pa, pi)
    }

    /// Masked interrupt-map lookup for a PCI (bus, dev, fn, pin): apply
    /// the node's `interrupt-map-mask` to key and entries alike, and
    /// return the matched tuple.  An absent mask, map, or match is a
    /// loud None — no default route is ever invented.
    pub fn interrupt_map_lookup(
        &self,
        node: Node,
        bus: u32,
        dev: u32,
        fun: u32,
        pin: u8,
    ) -> Option<PciIrqMap> {
        let mask = self.property(&node, "interrupt-map-mask")?;
        let mut cells = self.property_value_as_u32_iter(&mask);
        let mask_hi = cells.next()?;
        let mask_mid = cells.next()?;
        let mask_lo = cells.next()?;
        let mask_pin = cells.next()?;
        // PciIrqMap keys on phys.hi + pin only, so a mask routing on the
        // mid/lo cells is unsupported — loudly (the committed DTB's mask
        // is 0x1800/0/0/0x07).
        if mask_mid != 0 || mask_lo != 0 {
            return None;
        }
        let key_hi = (bus << 16) | (dev << 11) | (fun << 8);
        self.interrupt_map_iter(node).find(|e| {
            (e.child_hi & mask_hi) == (key_hi & mask_hi)
                && ((e.pin as u32) & mask_pin) == ((pin as u32) & mask_pin)
        })
    }

    fn property_value_contains(&self, prop: &Property, bytes_to_find: &str) -> bool {
        if let Some(init_value) = self.property_value_bytes(prop) {
            return init_value.split(|b| *b == b'\0').any(|bs| bs == bytes_to_find.as_bytes());
        }
        false
    }

    /// Return the node specified by the path, or None
    pub fn find_by_path(&self, path: &str) -> Option<Node> {
        fn find_subpath<'a, I>(
            dt: &DeviceTree,
            path_iter: &mut I,
            node: &Node,
            curr_path_element: Option<&str>,
        ) -> Option<Node>
        where
            I: Iterator<Item = &'a str>,
        {
            // Found the end of the path, so return the node
            let node_name = dt.node_name(node);
            if curr_path_element == node_name {
                let next_path_element = path_iter.next();
                if next_path_element.is_none() {
                    return Some(*node);
                }
                // Matching element on path, so recurse into children
                for child in dt.children(node) {
                    let found_node = find_subpath(dt, path_iter, &child, next_path_element);
                    if found_node.is_some() {
                        return found_node;
                    }
                }
            }

            None
        }

        // Prime the recursion with the first element of the path
        let mut path_iter = path.split_terminator('/');
        let next_path_element = path_iter.next();

        self.root().and_then(|node| find_subpath(self, &mut path_iter, &node, next_path_element))
    }

    /// Return the first node matching the compatible string 'comp'
    pub fn find_compatible(&'a self, comp: &'a str) -> impl Iterator<Item = Node> + 'a {
        // Iterate over all nodes.  For each node, iterate over all properties until we find a 'compatible'
        // property.  The 'compatible' property contains a list of null terminated strings.  If we find a matching
        // string, then return the node, otherwise return None.
        self.nodes().filter(|n| {
            if let Some(comp_prop) = self.property(n, "compatible") {
                return self.property_value_contains(&comp_prop, comp);
            }
            false
        })
    }

    /// Return iterator of nodes matching the device_type string 'device_type'
    pub fn find_device_type(&'a self, device_type: &'a str) -> impl Iterator<Item = Node> + 'a {
        self.nodes().filter(|n| {
            if let Some(prop) = self.property(n, "device_type") {
                return self.property_value_contains(&prop, device_type);
            }
            false
        })
    }

    fn inline_str(bytes: &[u8], start: usize) -> Option<&str> {
        let init_bytes = bytes.get(start..)?;
        let cstr = CStr::from_bytes_until_nul(init_bytes).ok()?;
        cstr.to_str().ok()
    }

    fn node_from_index(&self, start: usize, node_depth: usize) -> Option<Node> {
        // Iterate through data, finding the start index of the beginning of the
        // FDT_BEGIN_NODE token, and the index of the end of the FDT_END_NODE token.
        let structs = self.structs();
        let mut i = start;
        let mut begin_node_ctx: Option<FdtBeginNodeContext> = None;
        let mut next_token_start = 0;
        let mut depth = node_depth;

        while i < structs.len() {
            let token = Self::parse_token(structs, i);

            match token {
                Some(FdtToken::BeginNode(ctx)) => {
                    if depth == node_depth {
                        // Found the actual start of the next node
                        begin_node_ctx.replace(ctx);
                        next_token_start = i + ctx.total_len;
                    }
                    depth += 1;
                    i += ctx.total_len;
                }
                Some(FdtToken::EndNode(ctx)) => {
                    depth -= 1;
                    if depth == node_depth {
                        return begin_node_ctx.map(|begin_ctx| Node {
                            start: begin_ctx.start,
                            name_start: begin_ctx.name_start,
                            next_token_start,
                            total_len: (ctx.start + ctx.total_len) - begin_ctx.start,
                            depth: node_depth,
                        });
                    }
                    i += ctx.total_len;
                }
                Some(FdtToken::Prop(ctx)) => {
                    i += ctx.total_len;
                }
                Some(FdtToken::Nop(ctx) | FdtToken::End(ctx)) => {
                    i += ctx.total_len;
                }
                None => return None, // Shouldn't get here normally, so just None
            }
        }
        // Node returned at FDT_END_NODE
        None
    }

    /// Linearly iterate over the nodes in the order they occur in the flattened device tree
    pub fn nodes(&self) -> impl Iterator<Item = Node> + '_ {
        let structs = self.structs();
        let mut i = 0;
        let mut depth = 0;

        // On each iteration, i should be at or before the next node token we expect.
        // This is achieved by setting it to next_token_start when the end node token is found.
        // next_token_start is set when the first begin node token of the iteration is found, and
        // points to the next token after that begin node token.

        core::iter::from_fn(move || {
            let mut node_depth = 0;
            let mut next_token_start = 0;
            let mut begin_node_ctx: Option<FdtBeginNodeContext> = None;

            while i < structs.len() {
                let token = Self::parse_token(structs, i);
                if let Some(token) = token {
                    match token {
                        FdtToken::BeginNode(ctx) => {
                            if begin_node_ctx.is_none() {
                                begin_node_ctx.replace(ctx);
                                node_depth = depth;
                                next_token_start = i + ctx.total_len;
                            }
                            depth += 1;
                            i += ctx.total_len;
                        }

                        FdtToken::EndNode(ctx) => {
                            if begin_node_ctx.is_some() && (depth - 1) == node_depth {
                                // Reset i for the next node iteration
                                i = next_token_start;
                                let new_node = begin_node_ctx.take().map(|begin_ctx| Node {
                                    start: begin_ctx.start,
                                    name_start: begin_ctx.name_start,
                                    next_token_start,
                                    total_len: (ctx.start + ctx.total_len) - begin_ctx.start,
                                    depth: node_depth,
                                });
                                return new_node;
                            }

                            depth -= 1;
                            i += ctx.total_len;
                        }
                        FdtToken::Prop(ctx) => {
                            i += ctx.total_len;
                        }
                        FdtToken::Nop(ctx) | FdtToken::End(ctx) => {
                            i += ctx.total_len;
                        }
                    }
                } else {
                    return None;
                }
            }
            None
        })
    }

    /// Linearly iterate over the properties of a node in the order they occur in the flattened device tree
    fn properties(&self, node: &Node) -> impl Iterator<Item = Property> + '_ {
        let structs = self.structs();
        let end_i = node.start + node.total_len;
        let mut i = node.next_token_start;

        core::iter::from_fn(move || {
            while i < end_i {
                let token = Self::parse_token(structs, i);

                // Node properties come before any children
                match token {
                    Some(FdtToken::Prop(ctx)) => {
                        i += ctx.total_len;
                        return Some(Property {
                            start: ctx.start,
                            name_start: ctx.name_start,
                            value_start: ctx.value_start,
                            value_len: ctx.value_len,
                            total_len: ctx.total_len,
                        });
                    }
                    Some(FdtToken::Nop(ctx)) => {
                        i += ctx.total_len;
                    }
                    _ => return None,
                }
            }
            None
        })
    }

    fn parse_token(structs: &[u8], i: usize) -> Option<FdtToken> {
        let token = structs.get(i..).and_then(bytes_to_u32);

        match token {
            Some(0x1) => {
                // Null terminated string follow token
                let str_size = structs
                    .get((i + 4)..)
                    .and_then(|bs| bs.iter().position(|&b| b == 0))
                    .map(|sz| align4(sz + 1))
                    .unwrap_or(0);
                Some(FdtToken::BeginNode(FdtBeginNodeContext {
                    start: i,
                    name_start: i + 4,
                    total_len: 4 + str_size,
                }))
            }
            Some(0x2) => Some(FdtToken::EndNode(FdtTokenContext { start: i, total_len: 4 })),
            Some(0x3) => {
                let len = structs.get((i + 4)..).and_then(bytes_to_u32).unwrap_or(0);
                let nameoff = structs.get((i + 8)..).and_then(bytes_to_u32).unwrap_or(0);
                Some(FdtToken::Prop(FdtPropContext {
                    start: i,
                    name_start: nameoff as usize,
                    value_start: i + 12,
                    value_len: len as usize,
                    total_len: 12 + align4(len as usize),
                }))
            }
            Some(0x4) => Some(FdtToken::Nop(FdtTokenContext { start: i, total_len: 4 })),
            Some(0x9) => Some(FdtToken::End(FdtTokenContext { start: i, total_len: 4 })),
            _ => None,
        }
    }
}

/// Flattened Devicetree header structure, as documented in the spec
#[derive(Debug)]
#[allow(dead_code)]
struct FdtHeader {
    magic: u32,
    totalsize: u32,
    off_dt_struct: u32,
    off_dt_strings: u32,
    off_mem_rsvmap: u32,
    version: u32,
    last_comp_version: u32,
    boot_cpuid_phys: u32,
    size_dt_strings: u32,
    size_dt_struct: u32,
}

impl FdtHeader {
    /// Read FdtHeader from the stream, returning it in a Result if it passes validation,
    /// otherwise returns an error.
    /// Set ignore_size to true if you're only loading a portion of the buffer
    /// (e.g. in order to work out the size of the buffer before casting to a slice).
    fn new(data: &[u8], ignore_size: bool) -> Result<Self> {
        fn new_header(data: &[u8]) -> Option<FdtHeader> {
            Some(FdtHeader {
                magic: bytes_to_u32_offset(data, 0)?,
                totalsize: bytes_to_u32_offset(data, 4)?,
                off_dt_struct: bytes_to_u32_offset(data, 8)?,
                off_dt_strings: bytes_to_u32_offset(data, 12)?,
                off_mem_rsvmap: bytes_to_u32_offset(data, 16)?,
                version: bytes_to_u32_offset(data, 20)?,
                last_comp_version: bytes_to_u32_offset(data, 24)?,
                boot_cpuid_phys: bytes_to_u32_offset(data, 28)?,
                size_dt_strings: bytes_to_u32_offset(data, 32)?,
                size_dt_struct: bytes_to_u32_offset(data, 36)?,
            })
        }

        let len = data.len() as u32;
        new_header(data)
            .ok_or(ParseError::InvalidHeader)
            .and_then(|h| (h.magic == 0xd00dfeed).then_some(h).ok_or(ParseError::InvalidMagic))
            .and_then(|h| {
                (len == h.totalsize || ignore_size).then_some(h).ok_or(ParseError::BufferTooSmall)
            })
    }
}

/// Token represents one of 5 tokens in the FDT specification.  The names and IDs correspond
/// to those in the specification.
#[derive(Debug, Copy, Clone)]
enum FdtToken {
    BeginNode(FdtBeginNodeContext), // Start of a new node
    EndNode(FdtTokenContext),       // End of current node
    Prop(FdtPropContext),           // New property in current node
    Nop(FdtTokenContext),           // To be ignored
    End(FdtTokenContext),           // End of FDT structure
}

#[derive(Debug, Copy, Clone)]
struct FdtBeginNodeContext {
    start: usize,      // Start of token in buffer
    total_len: usize,  // Number of bytes for token, name and alignment
    name_start: usize, // Start of node name in sturcts buffer
}

#[derive(Debug, Copy, Clone)]
struct FdtPropContext {
    start: usize,       // Start of token in buffer
    total_len: usize,   // Number of bytes for token, len, nameoff, value and alignment
    name_start: usize,  // Start of node name in strings buffer
    value_start: usize, // Start of property value
    value_len: usize,   // Size of property value
}

#[derive(Debug, Copy, Clone)]
struct FdtTokenContext {
    start: usize,     // Start of token in buffer
    total_len: usize, // Number of bytes for token
}

#[derive(Debug, PartialEq, Copy, Clone)]
pub struct Node {
    start: usize,            // Start index in structs of node (Start of FDT_BEGIN_NODE)
    name_start: usize,       // Start of node name in structs buffer
    next_token_start: usize, // Start index of token after FDT_BEGIN_NODE
    total_len: usize,        // Total length of node
    depth: usize,            // Depth of node, with 0 being the root
}

impl Node {
    fn encloses(&self, child: &Node) -> bool {
        let parent_starts_before_child = self.start <= child.start;
        let parent_ends_after_child =
            (self.start + self.total_len) >= (child.start + child.total_len);
        parent_starts_before_child && parent_ends_after_child
    }

    fn is_root(&self) -> bool {
        self.depth == 0
    }

    pub fn depth(&self) -> usize {
        self.depth
    }
}

#[derive(Debug, PartialEq, Copy, Clone)]
pub struct Property {
    start: usize,       // Start index in structs of node (Start of FDT_BEGIN_NODE)
    name_start: usize,  // Start of node name in strings buffer
    value_start: usize, // Start index of property value
    value_len: usize,   // Size of property value
    total_len: usize,   // Total length of property
}

#[derive(Debug, PartialEq, Copy, Clone)]
pub struct RegBlock {
    pub addr: u64,
    pub len: Option<u64>,
}

impl RegBlock {
    pub fn from_addr(addr: u64) -> RegBlock {
        RegBlock { addr, len: None }
    }

    pub fn with_offset(self, offset: u64) -> RegBlock {
        RegBlock { addr: self.addr + offset, len: self.len }
    }
}

#[derive(Debug, PartialEq, Copy, Clone)]
pub enum TranslatedReg {
    Translated(RegBlock),
    Unreachable,
}

impl TranslatedReg {
    pub fn regblock(&self) -> Option<RegBlock> {
        match self {
            TranslatedReg::Translated(regblock) => Some(*regblock),
            _ => None,
        }
    }
}

#[derive(Debug, PartialEq, Copy, Clone)]
pub struct RangeMapping {
    pub child_bus_addr: u64,
    pub parent_bus_addr: u64,
    pub len: u64,
}

#[derive(Debug, PartialEq, Copy, Clone)]
pub enum Range {
    Identity,
    Translated(RangeMapping),
}

impl Range {
    /// Attempt to translate the given RegBlock.  If it can't be mapped, return None.
    fn translate(&self, r: RegBlock) -> Option<RegBlock> {
        match self {
            Range::Identity => Some(r),
            Range::Translated(map) => {
                if r.addr >= map.child_bus_addr && r.addr < map.child_bus_addr + map.len {
                    let addr = r.addr - map.child_bus_addr + map.parent_bus_addr;
                    return Some(RegBlock { addr, len: r.len });
                }
                None
            }
        }
    }
}

/// One decoded `interrupt-map` tuple (the PCI INTx swizzle table row).
/// The committed virt DTB's stride is 10 cells: 3 child-address +
/// 1 child-pin + 1 parent-phandle + 2 parent-address + 3 parent-specifier.
#[derive(Debug, PartialEq, Copy, Clone)]
pub struct PciIrqMap {
    /// Child unit-address phys.hi cell: `bus<<16 | dev<<11 | fn<<8`.
    pub child_hi: u32,
    /// Child interrupt specifier: the INTx pin (1 = INTA .. 4 = INTD).
    pub pin: u8,
    /// Interrupt-parent phandle (the GIC node on virt).
    pub parent_phandle: u32,
    /// Parent interrupt specifier cells (GIC: type, number, flags).
    pub parent_irq: [u32; 3],
}

/// Decode `bytes` as packed interrupt-map tuples of the given cell-count
/// layout (stride = child-addr + child-intr + 1 phandle + parent-addr +
/// parent-intr cells).  A zero stride or a tail shorter than one tuple
/// terminates iteration — never a panic, never a partial tuple (T-07-03).
fn decode_interrupt_map(
    bytes: &[u8],
    child_addr_cells: usize,
    child_intr_cells: usize,
    parent_addr_cells: usize,
    parent_intr_cells: usize,
) -> impl Iterator<Item = PciIrqMap> + '_ {
    let stride =
        (child_addr_cells + child_intr_cells + 1 + parent_addr_cells + parent_intr_cells) * 4;
    let mut i = 0usize;
    core::iter::from_fn(move || {
        if stride == 0 {
            return None;
        }
        let tuple = bytes.get(i..)?.get(..stride)?;
        i += stride;
        // In-bounds by construction: every index below is < stride/4.
        let cell = |n: usize| u32::from_be_bytes(tuple[n * 4..n * 4 + 4].try_into().unwrap());
        let child_hi = if child_addr_cells > 0 { cell(0) } else { 0 };
        let pin = if child_intr_cells > 0 { cell(child_addr_cells) as u8 } else { 0 };
        let parent_phandle = cell(child_addr_cells + child_intr_cells);
        let irq_base = child_addr_cells + child_intr_cells + 1 + parent_addr_cells;
        let mut parent_irq = [0u32; 3];
        for (k, slot) in parent_irq.iter_mut().enumerate() {
            if k < parent_intr_cells {
                *slot = cell(irq_base + k);
            }
        }
        Some(PciIrqMap { child_hi, pin, parent_phandle, parent_irq })
    })
}

// ---------------------------------------------------------------------------
// In-file decode tests over the committed qemu DTB blob
// (../aarch64/lib/virt-gicv2.dtb — the same bytes the kernel parses at
// boot).  Deliberate deviation from the crate-root test-placement
// precedent (allocator_tests/virtio_tests live at lib.rs because their
// arena/mock constructions need machinery that must stay out of the
// forbidden modules): the fdt decode tests need no unsafe and only a
// std::fs fixture read, and in-file placement keeps port/src/lib.rs
// untouched so sibling plans extending the module registry never conflict
// with this file-set.
// ---------------------------------------------------------------------------
#[cfg(test)]
mod tests {
    use super::*;

    /// The committed qemu `-M virt,gic-version=2` blob — the kernel's
    /// boot-time DTB, so a qemu drift reds this suite.
    fn virt_gicv2_dtb() -> Vec<u8> {
        let path = concat!(env!("CARGO_MANIFEST_DIR"), "/../aarch64/lib/virt-gicv2.dtb");
        std::fs::read(path).expect("committed virt-gicv2.dtb fixture")
    }

    fn pci_node(dt: &DeviceTree) -> Node {
        dt.find_device_type("pci").next().expect("pci node in committed DTB")
    }

    #[test]
    fn interrupt_map_decodes_all_16_entries() {
        let blob = virt_gicv2_dtb();
        let dt = DeviceTree::new(&blob).expect("header parses");
        let pci = pci_node(&dt);
        // The pci node publishes the cell counts the stride derives from.
        assert_eq!(dt.node_interrupt_cells(&pci), Some(1));
        let entries: Vec<PciIrqMap> = dt.interrupt_map_iter(pci).collect();
        assert_eq!(entries.len(), 16, "dev 0-3 x INTA-D");
        // Tuple 0: dev 0 INTA -> GIC SPI 3, level-high.
        assert_eq!(entries[0].child_hi, 0x0000);
        assert_eq!(entries[0].pin, 1);
        assert_eq!(entries[0].parent_phandle, 0x8002);
        assert_eq!(entries[0].parent_irq, [0, 3, 0x04]);
        // Tuple 4: dev 1 INTA — the standard boot's NIC — -> SPI 4.
        assert_eq!(entries[4].child_hi, 0x0800);
        assert_eq!(entries[4].parent_irq, [0, 4, 0x04]);
    }

    #[test]
    fn masked_lookup_routes_the_virtio_nic_inta_to_spi4() {
        let blob = virt_gicv2_dtb();
        let dt = DeviceTree::new(&blob).expect("header parses");
        let pci = pci_node(&dt);
        // The NIC sits at bus 0, dev 1, fn 0 on the standard qemu boot
        // and raises INTA (pin 1) -> GIC SPI 4, level-high (0x04).
        let route = dt.interrupt_map_lookup(pci, 0, 1, 0, 1);
        assert!(route.is_some(), "dev 1 INTA route present");
        let route = route.unwrap();
        assert_eq!(route.parent_irq[0], 0, "SPI type cell");
        assert_eq!(route.parent_irq[1], 4, "GIC SPI number (INTID = 32 + SPI)");
        assert_eq!(route.parent_irq[2], 0x04, "level-high flags");
    }

    #[test]
    fn swizzle_rotation_matches_the_decode_table() {
        let blob = virt_gicv2_dtb();
        let dt = DeviceTree::new(&blob).expect("header parses");
        let pci = pci_node(&dt);
        // The classic (dev + pin) rotation, dev 0-3 x INTA-D.
        let expected = [[3, 4, 5, 6], [4, 5, 6, 3], [5, 6, 3, 4], [6, 3, 4, 5]];
        for (dev, pins) in expected.iter().enumerate() {
            for (pin, spi) in pins.iter().enumerate() {
                let route = dt.interrupt_map_lookup(pci, 0, dev as u32, 0, pin as u8 + 1);
                assert!(route.is_some(), "dev {dev} pin {} routed", pin + 1);
                assert_eq!(route.unwrap().parent_irq[1], *spi, "dev {dev} pin {}", pin + 1);
                assert_eq!(route.unwrap().parent_irq[2], 0x04);
            }
        }
    }

    #[test]
    fn lookup_applies_the_mask_not_an_exact_compare() {
        let blob = virt_gicv2_dtb();
        let dt = DeviceTree::new(&blob).expect("header parses");
        let pci = pci_node(&dt);
        // interrupt-map-mask 0x1800/0/0/0x07: dev bits 0-1 and the pin
        // participate; fn bits and dev bit >= 2 do not.
        let base = dt.interrupt_map_lookup(pci, 0, 1, 0, 1).expect("dev 1 INTA");
        let via_fn = dt.interrupt_map_lookup(pci, 0, 1, 7, 1).expect("fn bits masked out");
        assert_eq!(via_fn.parent_irq, base.parent_irq);
        let via_dev_hi = dt.interrupt_map_lookup(pci, 0, 5, 0, 1).expect("dev bit 2 masked out");
        assert_eq!(via_dev_hi.parent_irq, base.parent_irq);
        // Masked bits still discriminate: dev 2 INTA routes to SPI 5.
        assert_eq!(dt.interrupt_map_lookup(pci, 0, 2, 0, 1).unwrap().parent_irq[1], 5);
        // A pin outside INTA-D has no entry at all.
        assert!(dt.interrupt_map_lookup(pci, 0, 1, 0, 5).is_none());
    }

    #[test]
    fn loud_absence_returns_none_never_a_default() {
        let blob = virt_gicv2_dtb();
        let dt = DeviceTree::new(&blob).expect("header parses");
        // Absent phandle -> None (no fallback node).
        assert!(dt.find_phandle(0xffff_ffff).is_none());
        // The memory node carries no #interrupt-cells -> None (no silent 1).
        let mem = dt.find_device_type("memory").next().expect("memory node");
        assert!(dt.node_interrupt_cells(&mem).is_none());
        // ...and no interrupt-map property -> an empty iterator.
        assert_eq!(dt.interrupt_map_iter(mem).count(), 0);
    }

    #[test]
    fn decode_terminates_on_a_partial_tail() {
        // 2.5 tuples at the committed DTB's stride (10 cells): the half
        // tuple terminates iteration without panic (T-07-03).
        let mut bytes = Vec::new();
        let push = |bytes: &mut Vec<u8>, cells: [u32; 10]| {
            for c in cells {
                bytes.extend_from_slice(&c.to_be_bytes());
            }
        };
        push(&mut bytes, [0x0000, 0, 0, 1, 0x8002, 0, 0, 0, 3, 4]);
        push(&mut bytes, [0x0800, 0, 0, 1, 0x8002, 0, 0, 0, 4, 4]);
        bytes.extend_from_slice(&[0u8; 12]); // a third tuple's prefix only
        let decoded: Vec<PciIrqMap> = decode_interrupt_map(&bytes, 3, 1, 2, 3).collect();
        assert_eq!(decoded.len(), 2);
        assert_eq!(decoded[1].child_hi, 0x0800);
        assert_eq!(decoded[1].parent_irq, [0, 4, 0x04]);
    }
}
