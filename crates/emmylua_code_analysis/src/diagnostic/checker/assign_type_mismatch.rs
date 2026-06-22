//! Assign type mismatch — pure salsa.

use emmylua_parser::{LuaAssignStat, LuaAst, LuaAstNode, LuaAstToken, LuaExpr, LuaIndexExpr, LuaIndexKey, LuaLocalStat, LuaNameExpr, LuaVarExpr, NumberResult};

use crate::semantic_model::SemanticModel;
use crate::{DiagnosticCode, LuaMemberKey, LuaType, SemanticDeclLevel};

use super::{DiagnosticContext, humanize_lint_type};

pub fn check(context: &mut DiagnosticContext, model: &SemanticModel) {
    for node in model.get_root().descendants::<LuaAst>() {
        match node {
            LuaAst::LuaAssignStat(a) => check_assign(context, model, &a),
            LuaAst::LuaLocalStat(l) => check_local(context, model, &l),
            _ => {}
        }
    }
}

fn check_assign(context: &mut DiagnosticContext, model: &SemanticModel, assign: &LuaAssignStat) {
    let (vars, exprs) = assign.get_var_and_expr_list();
    let Ok(value_types) = model.infer_expr_list_types(&exprs, Some(vars.len())) else { return };
    for (idx, var) in vars.iter().enumerate() {
        let Some(val) = value_types.get(idx).map(|(t, _)| t.clone()) else { continue };
        match var {
            LuaVarExpr::IndexExpr(ix) => {
                let Some(prefix) = ix.get_prefix_expr() else { continue };
                let Ok(prefix_ty) = model.infer_expr(prefix) else { continue };
                let Some(key) = ix.get_index_key() else { continue };
                let mk = key_to_member_key(model, &key);
                let target = model.infer_member_type(&prefix_ty, &mk).unwrap_or(LuaType::Unknown);
                check_mismatch(context, ix.get_range(), &val, &target);
            }
            LuaVarExpr::NameExpr(name) => {
                let Ok(target) = model.infer_expr(LuaExpr::NameExpr(name.clone())) else { continue };
                check_mismatch(context, name.get_range(), &val, &target);
            }
        }
    }
}

fn check_local(context: &mut DiagnosticContext, model: &SemanticModel, local: &LuaLocalStat) {
    let names: Vec<_> = local.get_local_name_list().collect();
    let exprs: Vec<_> = local.get_value_exprs().collect();
    let Ok(value_types) = model.infer_expr_list_types(&exprs, Some(names.len())) else { return };
    for (idx, name) in names.iter().enumerate() {
        let Some(val) = value_types.get(idx).map(|(t, _)| t.clone()) else { continue };
        let Ok(target) = model.infer_expr(LuaExpr::NameExpr(emmylua_parser::LuaNameExpr::cast(name.syntax().clone()).unwrap())) else { continue };
        let Some(nk) = name.get_name_token() else { continue };
        check_mismatch(context, nk.get_range(), &val, &target);
    }
}

fn check_mismatch(context: &mut DiagnosticContext, range: rowan::TextRange, source: &LuaType, target: &LuaType) {
    if target.is_unknown() || target.is_any() { return }
    let db = context.db;
    if crate::check_type_compact(db, source, target).is_err() {
        context.add_diagnostic(DiagnosticCode::AssignTypeMismatch, range,
            t!("Cannot assign `%{src}` to `%{dst}`.",
                src = humanize_lint_type(db, source),
                dst = humanize_lint_type(db, target)).to_string(), None);
    }
}

fn key_to_member_key(model: &SemanticModel, key: &LuaIndexKey) -> LuaMemberKey {
    match key {
        LuaIndexKey::Name(n) => LuaMemberKey::Name(smol_str::SmolStr::new(n.get_name_text())),
        LuaIndexKey::String(s) => LuaMemberKey::Name(smol_str::SmolStr::new(s.get_value())),
        LuaIndexKey::Integer(i) => match i.get_number_value() { NumberResult::Int(n) => LuaMemberKey::Integer(n), _ => LuaMemberKey::None },
        LuaIndexKey::Idx(i) => LuaMemberKey::Integer(*i as i64),
        LuaIndexKey::Expr(e) => { let ty = model.infer_expr(e.clone()).unwrap_or(LuaType::Unknown); LuaMemberKey::ExprType(ty) }
    }
}
