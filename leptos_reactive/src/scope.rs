use crate::{
    AnyEffect, AnyMemo, AnySignal, EffectId, EffectState, MemoId, MemoState, Runtime, SignalId,
    SignalState,
};
use elsa::FrozenVec;
use std::{
    any::{Any, TypeId},
    cell::RefCell,
    collections::HashMap,
    fmt::Debug,
};

#[must_use = "Scope will leak memory if the disposer function is never called"]
pub fn create_scope(f: impl FnOnce(Scope) + 'static) -> ScopeDisposer {
    let runtime = Box::leak(Box::new(Runtime::new()));
    runtime.create_scope(f, None)
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct Scope {
    pub(crate) runtime: &'static Runtime,
    pub(crate) id: ScopeId,
}

impl Scope {
    pub fn child_scope(self, f: impl FnOnce(Scope)) -> ScopeDisposer {
        //self.runtime.create_scope(f, Some(self))
        f(self);
        ScopeDisposer(Box::new(move || {}))
    }

    pub fn untrack<T>(&self, f: impl FnOnce() -> T) -> T {
        self.runtime.untrack(f)
    }
}

// Internals
impl Scope {
    pub(crate) fn push_signal<T>(&self, state: SignalState<T>) -> SignalId
    where
        T: Debug + 'static,
    {
        self.runtime.scope(self.id, |scope| {
            scope.signals.push(Box::new(state));
            SignalId(scope.signals.len() - 1)
        })
    }

    pub(crate) fn push_effect<T>(&self, state: EffectState<T>) -> EffectId
    where
        T: Debug + 'static,
    {
        self.runtime.scope(self.id, |scope| {
            scope.effects.push(Box::new(state));
            EffectId(scope.effects.len() - 1)
        })
    }

    pub(crate) fn push_memo<T>(&self, state: MemoState<T>) -> MemoId
    where
        T: Debug + 'static,
    {
        self.runtime.scope(self.id, |scope| {
            scope.memos.push(Box::new(state));
            MemoId(scope.memos.len() - 1)
        })
    }

    pub fn dispose(self) {
        // first, drop child scopes
        self.runtime.scope(self.id, |scope| {
            for id in scope.children.borrow().iter() {
                self.runtime.remove_scope(id)
            }
        })
        // removing from the runtime will drop this Scope, and all its Signals/Effects/Memos
    }
}

pub struct ScopeDisposer(pub(crate) Box<dyn FnOnce()>);

impl ScopeDisposer {
    pub fn dispose(self) {
        (self.0)()
    }
}

impl Debug for ScopeDisposer {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("ScopeDisposer").finish()
    }
}

slotmap::new_key_type! { pub(crate) struct ScopeId; }

pub(crate) struct ScopeState {
    pub(crate) parent: Option<Scope>,
    pub(crate) contexts: RefCell<HashMap<TypeId, Box<dyn Any>>>,
    pub(crate) children: RefCell<Vec<ScopeId>>,
    pub(crate) signals: FrozenVec<Box<dyn AnySignal>>,
    pub(crate) memos: FrozenVec<Box<dyn AnyMemo>>,
    pub(crate) effects: FrozenVec<Box<dyn AnyEffect>>,
}

impl Debug for ScopeState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ScopeState").finish()
    }
}

impl ScopeState {
    pub(crate) fn new(parent: Option<Scope>) -> Self {
        Self {
            parent,
            contexts: Default::default(),
            children: Default::default(),
            signals: Default::default(),
            memos: Default::default(),
            effects: Default::default(),
        }
    }
}