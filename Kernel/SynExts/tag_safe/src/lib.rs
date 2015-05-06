// tag_safe
//
// A linting plugin to flag calls to methods not marked "tag_safe"
// from methods marked "tag_safe".
//
// Author: John Hodge (thePowersGang/Mutabah)
//
// TODO: Support '#[tag_unsafe(type)]' which is used when a method has no marker
// - Allows default safe fallback, with upwards propagation.
//
#![crate_name="tag_safe"]
#![crate_type="dylib"]
#![feature(plugin_registrar, rustc_private, box_syntax)]

#[macro_use]
extern crate log;

extern crate syntax;
#[macro_use]
extern crate rustc;

use syntax::ast;
use syntax::visit;
use syntax::codemap::Span;
use rustc::lint::LintPassObject;
use rustc::plugin::Registry;
use rustc::lint::{Context, LintPass, LintArray};
use syntax::attr::AttrMetaMethods;
use rustc::middle::{def,ty};

declare_lint!(NOT_TAGGED_SAFE, Warn, "Warn about use of non-tagged methods within tagged function");

#[derive(Copy,Clone,Debug)]
enum SafetyType
{
	Safe,
	Unsafe,
	Unknown,
}

#[derive(Default)]
struct Pass
{
	/// Cache of flag types
	flag_types: Vec<String>,
	/// Node => (Type => IsSafe)
	flag_cache: ::rustc::util::nodemap::NodeMap< ::rustc::util::nodemap::FnvHashMap<usize, SafetyType> >,
	
	lvl: usize,
}

struct Visitor<'a, 'tcx: 'a, F: FnMut(&Span) + 'a>
{
	pass: &'a mut Pass,
    tcx: &'a ty::ctxt<'tcx>,
	name: &'a str,
	unknown_assume: bool,
	cb: F,
}

struct Indent(usize);
impl ::std::fmt::Display for Indent {
	fn fmt(&self, f: &mut ::std::fmt::Formatter) -> ::std::fmt::Result {
		for s in ::std::iter::repeat(" ").take(self.0) {
			try!(write!(f, "{}", s));
		}
		Ok( () )
	}
}

impl LintPass for Pass {
    fn get_lints(&self) -> LintArray {
        lint_array!(NOT_TAGGED_SAFE)
    }

    fn check_fn(&mut self, cx: &Context, _kind: visit::FnKind, _decl: &ast::FnDecl, body: &ast::Block, _: Span, id: ast::NodeId) {
        let attrs = cx.tcx.map.attrs(id);
        for ty in attrs.iter()
            .filter(|a| a.check_name("tag_safe"))
            .filter_map(|a| a.meta_item_list())
            .flat_map(|x| x.iter())
        {
            // Search body for calls to non safe methods
            let mut v = Visitor{
					pass: self, tcx: cx.tcx, name: &ty.name(),
					// - Assumes an untagged method is unsafe
					unknown_assume: false,
					cb: |span| {
							cx.span_lint(NOT_TAGGED_SAFE, *span,
								&format!("Calling {}-unsafe method from a #[tag_safe({})] method", ty.name(), ty.name())[..]
								);
						},
					};
            debug!("Method {:?} is marked safe '{}'", id, ty.name());
            visit::walk_block(&mut v, body);
        }
    }
}

impl Pass
{
	fn check_for_marker(tcx: &ty::ctxt, id: ast::NodeId, marker: &str, name: &str) -> bool
	{
		debug!("Checking for marker {}({}) on {:?}", marker, name, id);
		tcx.map.attrs(id).iter()
			.filter_map( |a| if a.check_name(marker) { a.meta_item_list() } else { None })
			.flat_map(|x| x.iter())
			.any(|a| a.name() == name)
	}
	
	fn crate_method_is_safe(&mut self, tcx: &ty::ctxt, node_id: ast::NodeId, name: &str, unknown_assume: bool) -> bool
	{
		// Obtain tag name ID (avoids storing a string in the map)
		let name_id = 
			match self.flag_types.iter().position(|a| *a == name)
			{
			Some(v) => v,
			None => {
				self.flag_types.push( String::from(name) );
				self.flag_types.len() - 1
				},
			};
		
		// Check cache first
		if let Some(&st) = self.flag_cache.get(&node_id).and_then(|a| a.get(&name_id))
		{
			match st
			{
			SafetyType::Safe => true,
			SafetyType::Unsafe => false,
			SafetyType::Unknown => unknown_assume,
			}
		}
		else
		{
			// Search for a safety marker, possibly recursing
			let is_safe =
				if Self::check_for_marker(tcx, node_id, "tag_safe", name) {
					true
				}
				else if Self::check_for_marker(tcx, node_id, "tag_unsafe", name) {
					false
				}
				else {
					// Cache this method as unknown (to prevent infinite recursion)
					self.flag_cache.entry(node_id)
						.or_insert(Default::default())
						.insert(name_id, SafetyType::Unknown)
						;
					
					match tcx.map.get(node_id)
					{
					syntax::ast_map::NodeItem(i) =>
						match i.node {
						ast::ItemFn(_, _, _, _, ref body) => {
							// Enumerate this function's code, recursively checking for a call to an unsafe method
							let mut is_safe = true;
							{
								let mut v = Visitor {
									pass: self, tcx: tcx, name: name,
									unknown_assume: true,
									cb: |_| { is_safe = false; }
									};
								visit::walk_block(&mut v, body);
							}
							is_safe
							},
						_ => unknown_assume,
						},
					syntax::ast_map::NodeImplItem(i) =>
						match i.node {
						ast::MethodImplItem(_, ref body) => {
							let mut is_safe = true;
							{
								let mut v = Visitor {
									pass: self, tcx: tcx, name: name,
									unknown_assume: true,
									cb: |_| { is_safe = false; }
									};
								visit::walk_block(&mut v, body);
							}
							is_safe
							},
						_ => unknown_assume,
						},
					v @ _ => {
						error!("Node ID {} points to non-item {:?}", node_id, v);
						unknown_assume
						}
					}
				};
			self.flag_cache.entry(node_id)
				.or_insert(Default::default())
				.insert(name_id, if is_safe { SafetyType::Safe } else { SafetyType::Unsafe })
				;
			is_safe
		}
	}
	
	/// Locate a #[tag_safe(<name>)] attribute on the passed item
	pub fn method_is_safe(&mut self, tcx: &ty::ctxt, id: ast::DefId, name: &str, unknown_assume: bool) -> bool
	{
		self.lvl += 1;
		debug!("{}Checking method {:?} (A {})", Indent(self.lvl), id, unknown_assume);
		let rv = if id.krate == 0
			{
				// Dummy assumption value, as none should be unknown until recursion starts
				self.crate_method_is_safe(tcx, id.node, name, unknown_assume)
			}
			else
			{
				error!("TODO: Crate ID non-zero {:?} (assuming safe)", id);
				true
			};
		debug!("{}Checking method {:?} = {}", Indent(self.lvl), id, rv);
		self.lvl -= 1;
		rv
	}
}

impl<'a, 'tcx: 'a, F: FnMut(&Span)> visit::Visitor<'a> for Visitor<'a,'tcx, F>
{
    // - visit_expr_post doesn't need to do anything
    fn visit_expr_post(&mut self, ex: &'a ast::Expr) {
        match ex.node
        {
        ast::ExprCall(ref fcn, _) =>
            match fcn.node
            {
            ast::ExprPath(ref _qs, ref _p) => {
                    if let def::DefFn(did, _) = ty::resolve_expr(self.tcx, &fcn) {
						if !self.pass.method_is_safe(self.tcx, did, self.name, self.unknown_assume)
						{
							(self.cb)(&ex.span);
						}
                    }
                },
            _ => {},
            },
        ast::ExprMethodCall(ref id, ref _tys, ref exprs) => {
					let obj_node_id = exprs[0].id;
                    debug!("Call method {:?} {:?}", id.node, obj_node_id);
					let mm = self.tcx.method_map.borrow();
					match mm.get( &ty::MethodCall::expr(ex.id) ).unwrap().origin
					{
					ty::MethodStatic(id) => {
							if !self.pass.method_is_safe(self.tcx, id, self.name, self.unknown_assume) {
								(self.cb)(&ex.span);
							}
						},
					_ => {},
					}
                },
        _ => {},
        }
    }
}

#[plugin_registrar]
pub fn plugin_registrar(reg: &mut Registry) {
    reg.register_lint_pass(box Pass::default() as LintPassObject);
}


