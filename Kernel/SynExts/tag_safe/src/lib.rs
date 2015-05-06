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

declare_lint!(TAGGED_SAFE_LINT, Warn, "Warn about use of non-tagged methods within tagged function");

struct Pass;

struct Visitor<'a, 'tcx: 'a>
{
    lcx: &'a Context<'a, 'tcx>,
	name: &'a str,
}

impl LintPass for Pass {
    fn get_lints(&self) -> LintArray {
        lint_array!(TAGGED_SAFE_LINT)
    }

    fn check_fn(&mut self, cx: &Context, _: visit::FnKind, decl: &ast::FnDecl, body: &ast::Block, _: Span, id: ast::NodeId) {
        let attrs = cx.tcx.map.attrs(id);
        for ty in attrs.iter()
            .filter(|a| a.check_name("tag_safe"))
            .filter_map(|a| a.meta_item_list())
            .flat_map(|x| x.iter())
        {
            // Search body for calls to non safe methods
            let mut v = Visitor{ lcx: cx, name: &ty.name() };
            debug!("Method {:?} is marked safe '{}'", id, ty.name());
            visit::walk_block(&mut v, body);
        }
    }
}

/// Locate a #[tag_safe(<name>)] attribute on the passed item
fn method_is_safe(tcx: &ty::ctxt, id: ast::DefId, name: &str) -> bool
{
	if id.krate == 1
	{
		tcx.map.attrs(id.node).iter().any( |a|
			a.check_name("tag_safe")
			&& a.meta_item_list().iter().flat_map(|x| x.iter()).any(|a| a.name() == name)
			)
	}
	else
	{
		error!("TODO: Crate ID non-zero {:?}", id);
		false
	}
}

impl<'a, 'tcx: 'a> visit::Visitor<'a> for Visitor<'a,'tcx>
{
    // - visit_expr_post doesn't need to do anything
    fn visit_expr_post(&mut self, ex: &'a ast::Expr) {
        match ex.node
        {
        ast::ExprCall(ref fcn, _) =>
            match fcn.node
            {
            ast::ExprPath(ref qs, ref p) => {
                    if let def::DefFn(did, _) = ty::resolve_expr(self.lcx.tcx, &fcn) {
						let is_safe = method_is_safe(self.lcx.tcx, did, self.name);
						if !is_safe
						{
							self.lcx.span_lint(TAGGED_SAFE_LINT, ex.span,
								&format!("Calling untagged method from a #[tag_safe({})] method", self.name)[..]
								);
						}
                    }
                },
            _ => {},
            },
        ast::ExprMethodCall(ref id, ref tys, ref exprs) => {
					let obj_node_id = exprs[0].id;
                    debug!("Call method {:?} {:?}", id, obj_node_id);
					let mm = self.lcx.tcx.method_map.borrow();
					match mm.get( &ty::MethodCall::expr(ex.id) ).unwrap().origin
					{
					ty::MethodStatic(id) => {
							if !method_is_safe(self.lcx.tcx, id, self.name) {
								self.lcx.span_lint(TAGGED_SAFE_LINT, ex.span,
									&format!("Calling untagged method from a #[tag_safe({})] method", self.name)[..]
									);
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
    reg.register_lint_pass(box Pass as LintPassObject);
}


