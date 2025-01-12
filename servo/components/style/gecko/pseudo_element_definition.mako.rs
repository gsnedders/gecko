/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/. */

/// Gecko's pseudo-element definition.
#[derive(Clone, Debug, Eq, Hash, MallocSizeOf, PartialEq)]
pub enum PseudoElement {
    % for pseudo in PSEUDOS:
        /// ${pseudo.value}
        % if pseudo.is_tree_pseudo_element():
        ${pseudo.capitalized_pseudo()}(ThinBoxedSlice<Atom>),
        % else:
        ${pseudo.capitalized_pseudo()},
        % endif
    % endfor
    /// ::-webkit-* that we don't recognize
    /// https://github.com/whatwg/compat/issues/103
    UnknownWebkit(Atom),
}

/// Important: If you change this, you should also update Gecko's
/// nsCSSPseudoElements::IsEagerlyCascadedInServo.
<% EAGER_PSEUDOS = ["Before", "After", "FirstLine", "FirstLetter"] %>
<% TREE_PSEUDOS = [pseudo for pseudo in PSEUDOS if pseudo.is_tree_pseudo_element()] %>
<% SIMPLE_PSEUDOS = [pseudo for pseudo in PSEUDOS if not pseudo.is_tree_pseudo_element()] %>

/// The number of eager pseudo-elements.
pub const EAGER_PSEUDO_COUNT: usize = ${len(EAGER_PSEUDOS)};

/// The number of non-functional pseudo-elements.
pub const SIMPLE_PSEUDO_COUNT: usize = ${len(SIMPLE_PSEUDOS)};

/// The number of tree pseudo-elements.
pub const TREE_PSEUDO_COUNT: usize = ${len(TREE_PSEUDOS)};

/// The number of all pseudo-elements.
pub const PSEUDO_COUNT: usize = ${len(PSEUDOS)};

/// The list of eager pseudos.
pub const EAGER_PSEUDOS: [PseudoElement; EAGER_PSEUDO_COUNT] = [
    % for eager_pseudo_name in EAGER_PSEUDOS:
    PseudoElement::${eager_pseudo_name},
    % endfor
];

<%def name="pseudo_element_variant(pseudo, tree_arg='..')">\
PseudoElement::${pseudo.capitalized_pseudo()}${"({})".format(tree_arg) if pseudo.is_tree_pseudo_element() else ""}\
</%def>

impl PseudoElement {
    /// Get the pseudo-element as an atom.
    #[inline]
    fn atom(&self) -> Atom {
        match *self {
            % for pseudo in PSEUDOS:
                ${pseudo_element_variant(pseudo)} => atom!("${pseudo.value}"),
            % endfor
            PseudoElement::UnknownWebkit(..) => unreachable!(),
        }
    }

    /// Returns an index of the pseudo-element.
    #[inline]
    pub fn index(&self) -> usize {
        match *self {
            % for i, pseudo in enumerate(PSEUDOS):
            ${pseudo_element_variant(pseudo)} => ${i},
            % endfor
            PseudoElement::UnknownWebkit(..) => unreachable!(),
        }
    }

    /// Returns an array of `None` values.
    ///
    /// FIXME(emilio): Integer generics can't come soon enough.
    pub fn pseudo_none_array<T>() -> [Option<T>; PSEUDO_COUNT] {
        [
            ${",\n            ".join(["None" for pseudo in PSEUDOS])}
        ]
    }

    /// Whether this pseudo-element is an anonymous box.
    #[inline]
    pub fn is_anon_box(&self) -> bool {
        match *self {
            % for pseudo in PSEUDOS:
                % if pseudo.is_anon_box():
                    ${pseudo_element_variant(pseudo)} => true,
                % endif
            % endfor
            _ => false,
        }
    }

    /// Whether this pseudo-element is eagerly-cascaded.
    #[inline]
    pub fn is_eager(&self) -> bool {
        matches!(*self,
                 ${" | ".join(map(lambda name: "PseudoElement::{}".format(name), EAGER_PSEUDOS))})
    }

    /// Whether this pseudo-element is tree pseudo-element.
    #[inline]
    pub fn is_tree_pseudo_element(&self) -> bool {
        match *self {
            % for pseudo in TREE_PSEUDOS:
            ${pseudo_element_variant(pseudo)} => true,
            % endfor
            _ => false,
        }
    }

    /// Whether this pseudo-element is an unknown Webkit-prefixed pseudo-element.
    #[inline]
    pub fn is_unknown_webkit_pseudo_element(&self) -> bool {
        matches!(*self, PseudoElement::UnknownWebkit(..))
    }

    /// Gets the flags associated to this pseudo-element, or 0 if it's an
    /// anonymous box.
    pub fn flags(&self) -> u32 {
        match *self {
            % for pseudo in PSEUDOS:
                ${pseudo_element_variant(pseudo)} =>
                % if pseudo.is_tree_pseudo_element():
                    if unsafe { structs::StaticPrefs_sVarCache_layout_css_xul_tree_pseudos_content_enabled } {
                        0
                    } else {
                        structs::CSS_PSEUDO_ELEMENT_ENABLED_IN_UA_SHEETS_AND_CHROME
                    },
                % elif pseudo.is_anon_box():
                    structs::CSS_PSEUDO_ELEMENT_ENABLED_IN_UA_SHEETS,
                % else:
                    structs::SERVO_CSS_PSEUDO_ELEMENT_FLAGS_${pseudo.pseudo_ident},
                % endif
            % endfor
            PseudoElement::UnknownWebkit(..) => 0,
        }
    }

    /// Construct a pseudo-element from a `CSSPseudoElementType`.
    #[inline]
    pub fn from_pseudo_type(type_: CSSPseudoElementType) -> Option<Self> {
        match type_ {
            % for pseudo in PSEUDOS:
                % if not pseudo.is_anon_box():
                    CSSPseudoElementType::${pseudo.pseudo_ident} => {
                        Some(${pseudo_element_variant(pseudo)})
                    },
                % endif
            % endfor
            _ => None,
        }
    }

    /// Construct a `CSSPseudoElementType` from a pseudo-element
    #[inline]
    fn pseudo_type(&self) -> CSSPseudoElementType {
        use crate::gecko_bindings::structs::CSSPseudoElementType_InheritingAnonBox;

        match *self {
            % for pseudo in PSEUDOS:
                % if not pseudo.is_anon_box():
                    PseudoElement::${pseudo.capitalized_pseudo()} => CSSPseudoElementType::${pseudo.pseudo_ident},
                % elif pseudo.is_tree_pseudo_element():
                    PseudoElement::${pseudo.capitalized_pseudo()}(..) => CSSPseudoElementType::XULTree,
                % elif pseudo.is_inheriting_anon_box():
                    PseudoElement::${pseudo.capitalized_pseudo()} => CSSPseudoElementType_InheritingAnonBox,
                % else:
                    PseudoElement::${pseudo.capitalized_pseudo()} => CSSPseudoElementType::NonInheritingAnonBox,
                % endif
            % endfor
            PseudoElement::UnknownWebkit(..) => unreachable!(),
        }
    }

    /// Get a PseudoInfo for a pseudo
    pub fn pseudo_info(&self) -> (*mut structs::nsAtom, CSSPseudoElementType) {
        (self.atom().as_ptr(), self.pseudo_type())
    }

    /// Get the argument list of a tree pseudo-element.
    #[inline]
    pub fn tree_pseudo_args(&self) -> Option<<&[Atom]> {
        match *self {
            % for pseudo in TREE_PSEUDOS:
            PseudoElement::${pseudo.capitalized_pseudo()}(ref args) => Some(args),
            % endfor
            _ => None,
        }
    }

    /// Construct a pseudo-element from an `Atom`.
    #[inline]
    pub fn from_atom(atom: &Atom) -> Option<Self> {
        % for pseudo in PSEUDOS:
            % if pseudo.is_tree_pseudo_element():
                // We cannot generate ${pseudo_element_variant(pseudo)} from just an atom.
            % else:
                if atom == &atom!("${pseudo.value}") {
                    return Some(${pseudo_element_variant(pseudo)});
                }
            % endif
        % endfor
        None
    }

    /// Construct a pseudo-element from an anonymous box `Atom`.
    #[inline]
    pub fn from_anon_box_atom(atom: &Atom) -> Option<Self> {
        % for pseudo in PSEUDOS:
            % if pseudo.is_tree_pseudo_element():
                // We cannot generate ${pseudo_element_variant(pseudo)} from just an atom.
            % elif pseudo.is_anon_box():
                if atom == &atom!("${pseudo.value}") {
                    return Some(${pseudo_element_variant(pseudo)});
                }
            % endif
        % endfor
        None
    }

    /// Construct a tree pseudo-element from atom and args.
    #[inline]
    pub fn from_tree_pseudo_atom(atom: &Atom, args: Box<[Atom]>) -> Option<Self> {
        % for pseudo in PSEUDOS:
            % if pseudo.is_tree_pseudo_element():
                if atom == &atom!("${pseudo.value}") {
                    return Some(PseudoElement::${pseudo.capitalized_pseudo()}(args.into()));
                }
            % endif
        % endfor
        None
    }

    /// Constructs a pseudo-element from a string of text.
    ///
    /// Returns `None` if the pseudo-element is not recognised.
    #[inline]
    pub fn from_slice(name: &str) -> Option<Self> {
        // We don't need to support tree pseudos because functional
        // pseudo-elements needs arguments, and thus should be created
        // via other methods.
        match_ignore_ascii_case! { name,
            % for pseudo in SIMPLE_PSEUDOS:
            "${pseudo.value[1:]}" => {
                return Some(${pseudo_element_variant(pseudo)})
            }
            % endfor
            // Alias "-moz-selection" to "selection" at parse time.
            "-moz-selection" => {
                return Some(PseudoElement::Selection);
            }
            "-moz-placeholder" => {
                return Some(PseudoElement::Placeholder);
            }
            _ => {
                if starts_with_ignore_ascii_case(name, "-moz-tree-") {
                    return PseudoElement::tree_pseudo_element(name, Box::new([]))
                }
                if unsafe {
                    structs::StaticPrefs_sVarCache_layout_css_unknown_webkit_pseudo_element
                } {
                    const WEBKIT_PREFIX: &str = "-webkit-";
                    if starts_with_ignore_ascii_case(name, WEBKIT_PREFIX) {
                        let part = string_as_ascii_lowercase(&name[WEBKIT_PREFIX.len()..]);
                        return Some(PseudoElement::UnknownWebkit(part.into()));
                    }
                }
            }
        }

        None
    }

    /// Constructs a tree pseudo-element from the given name and arguments.
    /// "name" must start with "-moz-tree-".
    ///
    /// Returns `None` if the pseudo-element is not recognized.
    #[inline]
    pub fn tree_pseudo_element(name: &str, args: Box<[Atom]>) -> Option<Self> {
        debug_assert!(starts_with_ignore_ascii_case(name, "-moz-tree-"));
        let tree_part = &name[10..];
        % for pseudo in TREE_PSEUDOS:
            if tree_part.eq_ignore_ascii_case("${pseudo.value[11:]}") {
                return Some(${pseudo_element_variant(pseudo, "args.into()")});
            }
        % endfor
        None
    }
}

impl ToCss for PseudoElement {
    fn to_css<W>(&self, dest: &mut W) -> fmt::Result where W: fmt::Write {
        dest.write_char(':')?;
        match *self {
            % for pseudo in PSEUDOS:
                ${pseudo_element_variant(pseudo)} => dest.write_str("${pseudo.value}")?,
            % endfor
            PseudoElement::UnknownWebkit(ref atom) => {
                dest.write_str(":-webkit-")?;
                serialize_atom_identifier(atom, dest)?;
            }
        }
        if let Some(args) = self.tree_pseudo_args() {
            if !args.is_empty() {
                dest.write_char('(')?;
                let mut iter = args.iter();
                if let Some(first) = iter.next() {
                    serialize_atom_identifier(&first, dest)?;
                    for item in iter {
                        dest.write_str(", ")?;
                        serialize_atom_identifier(item, dest)?;
                    }
                }
                dest.write_char(')')?;
            }
        }
        Ok(())
    }
}
