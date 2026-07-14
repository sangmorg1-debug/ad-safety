#![feature(rustc_private)]

extern crate rustc_ast;
extern crate rustc_driver;
extern crate rustc_errors;
extern crate rustc_hir;
extern crate rustc_interface;
extern crate rustc_middle;
extern crate rustc_session;
extern crate rustc_span;

use rustc_driver::Compilation;
use rustc_hir::def_id::LocalDefId;
use rustc_interface::interface::Compiler;
use rustc_middle::mir;
use rustc_middle::ty::{self, TyCtxt};
use rustc_span::Span;

pub struct AdSafetyCallbacks;

impl rustc_driver::Callbacks for AdSafetyCallbacks {
    fn after_analysis<'tcx>(
        &mut self,
        _compiler: &Compiler,
        tcx: TyCtxt<'tcx>,
    ) -> Compilation {
        run_ad_safety_analysis(tcx);
        Compilation::Continue
    }
}

fn run_ad_safety_analysis(tcx: TyCtxt<'_>) {
    // Iterate over all items with MIR in the current crate
    for &local_def_id in tcx.mir_keys(()) {
        check_function(tcx, local_def_id);
    }
}

fn check_function(tcx: TyCtxt<'_>, def_id: LocalDefId) {
    let hir_id = tcx.local_def_id_to_hir_id(def_id);
    let attrs = tcx.hir_attrs(hir_id);

    for attr in attrs {
        if let rustc_hir::Attribute::Parsed(rustc_hir::attrs::AttributeKind::RustcAutodiff(ref autodiff_opt)) = attr {
            let span = tcx.def_span(def_id);
            match autodiff_opt {
                Some(autodiff) => {
                    if autodiff.mode == rustc_ast::expand::autodiff_attrs::DiffMode::Reverse {
                        // This is the generated derivative function (df). Rule 1 check.
                        audit_reverse_signature(tcx, def_id, span, autodiff);
                    }
                }
                None => {
                    // This is the primal source function. Rule 2 check.
                    audit_primal_body(tcx, def_id, span);
                }
            }
        }
    }
}

fn audit_reverse_signature(
    tcx: TyCtxt<'_>,
    def_id: LocalDefId,
    attr_span: Span,
    autodiff: &rustc_hir::attrs::RustcAutodiff,
) {
    let session = tcx.sess;
    let fn_sig = tcx.fn_sig(def_id).instantiate_identity().skip_normalization();
    let inputs = fn_sig.skip_binder().inputs();

    for (i, &input_ty) in inputs.iter().enumerate() {
        if i >= autodiff.input_activity.len() {
            break;
        }

        let activity = autodiff.input_activity[i];
        if activity == rustc_ast::expand::autodiff_attrs::DiffActivity::Active {
            let is_ptr_or_ref = match input_ty.kind() {
                ty::TyKind::Ref(..) | ty::TyKind::RawPtr(..) => true,
                _ => false,
            };

            if is_ptr_or_ref {
                let param_span = tcx.def_span(def_id);
                #[allow(deprecated)]
                let mut diag = session.dcx().struct_span_err(
                    param_span,
                    format!(
                        "parameter {} has type `{}` but is marked as `Active` in reverse-mode autodiff",
                        i + 1,
                        input_ty
                    ),
                );
                diag.span_label(attr_span, "differentiated here");
                diag.help("reference and pointer parameters must be marked as `Duplicated` (not `Active`) in reverse mode. Change the activity to `Duplicated` and pass a shadow parameter to receive the gradient in-place.");
                diag.emit();
            }
        }
    }
}

fn audit_primal_body(
    tcx: TyCtxt<'_>,
    def_id: LocalDefId,
    attr_span: Span,
) {
    let session = tcx.sess;
    let body = tcx.optimized_mir(def_id);

    // Scan local declarations for coroutine types
    for local_decl in &body.local_decls {
        let ty = local_decl.ty;
        if let ty::TyKind::Coroutine(..) = ty.kind() {
            let var_span = local_decl.source_info.span;
            #[allow(deprecated)]
            let mut diag = session.dcx().struct_span_err(
                var_span,
                "use of coroutines/generators is not supported in differentiated functions",
            );
            diag.span_label(attr_span, "inside this differentiated function");
            diag.note("differentiating through coroutine state machines silently ignores state mutation, leading to incorrect gradients (dx: 0)");
            diag.emit();
        }
    }

    // Scan terminators for calls returning coroutines
    for bb in body.basic_blocks.iter() {
        if let mir::TerminatorKind::Call { func, .. } = &bb.terminator().kind {
            let func_ty = func.ty(&body.local_decls, tcx);
            if let ty::TyKind::FnDef(callee_def_id, _) = func_ty.kind() {
                let callee_sig = tcx.fn_sig(*callee_def_id).instantiate_identity().skip_normalization();
                let output_ty = callee_sig.skip_binder().output();
                if let ty::TyKind::Coroutine(..) = output_ty.kind() {
                    let call_span = bb.terminator().source_info.span;
                    #[allow(deprecated)]
                    let mut diag = session.dcx().struct_span_err(
                        call_span,
                        "calling functions that return coroutines/generators is not supported in differentiated functions",
                    );
                    diag.span_label(attr_span, "inside this differentiated function");
                    diag.emit();
                }
            }
        }
    }
}
