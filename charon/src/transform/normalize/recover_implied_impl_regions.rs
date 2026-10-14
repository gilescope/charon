//! Recover the regions hax erases in the impl exprs proving an impl's implied clauses.
//!
//! `impl<'a, T> DoubleEndedIterator for IterMut<'a, T>` proves its parent clause
//! `IterMut<'a, T>: Iterator` by `impl_Iterator_for_IterMut<'_, T>`: trait resolution works on
//! erased regions, so the proof's arguments carry none. Everything read through that proof then
//! loses them too - the lifted `Item` of the `DoubleEndedIterator` impl becomes `&'_ mut T`,
//! which a consumer cannot tie to `'a` (Aeneas gives nothing back through `Rev<IterMut>::next`).
//!
//! The regions are recoverable: the referenced impl's header (`Iterator for IterMut<'r, T>`)
//! matched against the clause it proves (`IterMut<'a, T>: Iterator`) binds `'r := 'a`. This
//! fills each erased argument from that binding and leaves the others erased.
use crate::ast::*;
use crate::transform::{TransformCtx, ctx::TransformPass};

/// Bind the regions of `pattern` (the referenced impl's own parameters, bound at depth 0) to
/// those of `target`, walking both in step.
fn match_ty(pattern: &Ty, target: &Ty, bindings: &mut IndexVec<RegionId, Option<Region>>) {
    match (pattern.kind(), target.kind()) {
        (TyKind::Ref(rp, tp, _), TyKind::Ref(rt, tt, _)) => {
            match_region(rp, rt, bindings);
            match_ty(tp, tt, bindings);
        }
        (TyKind::Adt(p), TyKind::Adt(t)) if p.id == t.id => {
            match_args(&p.generics, &t.generics, bindings)
        }
        (TyKind::Slice(tp, _), TyKind::Slice(tt, _))
        | (TyKind::RawPtr(tp, _), TyKind::RawPtr(tt, _))
        | (TyKind::Array(tp, _, _), TyKind::Array(tt, _, _)) => match_ty(tp, tt, bindings),
        _ => {}
    }
}

fn match_region(pattern: &Region, target: &Region, bindings: &mut IndexVec<RegionId, Option<Region>>) {
    if let Region::Var(DeBruijnVar::Bound(depth, id)) = pattern
        && depth.is_zero()
        && !matches!(target, Region::Erased)
        && let Some(slot) = bindings.get_mut(*id)
        && slot.is_none()
    {
        *slot = Some(target.clone());
    }
}

fn match_args(pattern: &GenericArgs, target: &GenericArgs, bindings: &mut IndexVec<RegionId, Option<Region>>) {
    for (rp, rt) in pattern.regions.iter().zip(target.regions.iter()) {
        match_region(rp, rt, bindings);
    }
    for (tp, tt) in pattern.types.iter().zip(target.types.iter()) {
        match_ty(tp, tt, bindings);
    }
}

pub struct Transform;
impl TransformPass for Transform {
    fn transform_ctx(&self, ctx: &mut TransformCtx) {
        let impls = &ctx.translated.trait_impls;
        let mut fixes: Vec<(TraitImplId, TraitClauseId, TraitRef)> = Vec::new();
        for timpl in impls {
            for (clause_id, tref) in timpl.implied_trait_refs.iter_enumerated() {
                let TraitRefKind::TraitImpl(impl_ref) = &tref.kind else {
                    continue;
                };
                if !impl_ref.generics.regions.iter().any(|r| matches!(r, Region::Erased)) {
                    continue;
                }
                let Some(target) = impls.get(impl_ref.id) else {
                    continue;
                };
                // The clause proved, moved out of its own (empty) region binder into the impl's
                // context, where the proof's arguments live.
                let Some(pred) = tref.trait_decl_ref.skip_binder.clone().move_from_under_binder()
                else {
                    continue;
                };
                let mut bindings: IndexVec<RegionId, Option<Region>> =
                    target.generics.regions.map_ref(|_| None);
                match_args(&target.impl_trait.generics, &pred.generics, &mut bindings);
                let mut generics = (*impl_ref.generics).clone();
                let mut changed = false;
                for (id, region) in generics.regions.iter_mut_enumerated() {
                    if matches!(region, Region::Erased)
                        && let Some(Some(bound)) = bindings.get(id)
                    {
                        *region = bound.clone();
                        changed = true;
                    }
                }
                if changed {
                    let kind = TraitRefKind::TraitImpl(TraitImplRef {
                        id: impl_ref.id,
                        generics: Box::new(generics),
                    });
                    fixes.push((timpl.def_id, clause_id, TraitRef::new(kind, tref.trait_decl_ref.clone())));
                }
            }
        }
        for (impl_id, clause_id, tref) in fixes {
            if let Some(timpl) = ctx.translated.trait_impls.get_mut(impl_id) {
                timpl.implied_trait_refs[clause_id] = tref;
            }
        }
    }
}
