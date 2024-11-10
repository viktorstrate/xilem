use crate::{
    core::{
        AppendVec, DynMessage, ElementSplice, Mut, SuperElement, View, ViewElement, ViewMarker,
        ViewSequence,
    },
    Pod, ViewCtx,
};
use masonry::{
    widget::{self, WidgetMut},
    Widget,
};

pub fn zstack<State, Action, Seq: ZStackSequence<State, Action>>(sequence: Seq) -> Stack<Seq> {
    Stack { sequence }
}

pub struct Stack<Seq> {
    sequence: Seq,
}

impl<Seq> ViewMarker for Stack<Seq> {}
impl<State, Action, Seq> View<State, Action, ViewCtx> for Stack<Seq>
where
    State: 'static,
    Action: 'static,
    Seq: ZStackSequence<State, Action>,
{
    type Element = Pod<widget::ZStack>;

    type ViewState = Seq::SeqState;

    fn build(&self, ctx: &mut ViewCtx) -> (Self::Element, Self::ViewState) {
        let mut elements = AppendVec::default();
        let mut widget = widget::ZStack::new();
        let seq_state = self.sequence.seq_build(ctx, &mut elements);
        for child in elements.into_inner() {
            widget = widget.with_child_pod(child.0.inner);
        }
        (ctx.new_pod(widget), seq_state)
    }

    fn rebuild(
        &self,
        prev: &Self,
        view_state: &mut Self::ViewState,
        ctx: &mut ViewCtx,
        element: crate::core::Mut<Self::Element>,
    ) {
        let mut splice = StackSplice::new(element);
        self.sequence
            .seq_rebuild(&prev.sequence, view_state, ctx, &mut splice);
        debug_assert!(splice.scratch.is_empty());
    }

    fn teardown(
        &self,
        view_state: &mut Self::ViewState,
        ctx: &mut ViewCtx,
        element: crate::core::Mut<Self::Element>,
    ) {
        let mut splice = StackSplice::new(element);
        self.sequence.seq_teardown(view_state, ctx, &mut splice);
        debug_assert!(splice.scratch.into_inner().is_empty());
    }

    fn message(
        &self,
        view_state: &mut Self::ViewState,
        id_path: &[crate::core::ViewId],
        message: DynMessage,
        app_state: &mut State,
    ) -> crate::core::MessageResult<Action, DynMessage> {
        self.sequence
            .seq_message(view_state, id_path, message, app_state)
    }
}

// MARK: ZStackElement
pub struct ZStackElement(Pod<Box<dyn Widget>>);

pub struct ZStackElementMut<'w> {
    parent: WidgetMut<'w, widget::ZStack>,
    idx: usize,
}

impl ViewElement for ZStackElement {
    type Mut<'a> = ZStackElementMut<'a>;
}

impl SuperElement<ZStackElement, ViewCtx> for ZStackElement {
    fn upcast(_ctx: &mut ViewCtx, child: ZStackElement) -> Self {
        child
    }

    fn with_downcast_val<R>(
        mut this: crate::core::Mut<Self>,
        f: impl FnOnce(crate::core::Mut<ZStackElement>) -> R,
    ) -> (Self::Mut<'_>, R) {
        let r = {
            let parent = this.parent.reborrow_mut();
            let reborrow = ZStackElementMut {
                idx: this.idx,
                parent,
            };
            f(reborrow)
        };
        (this, r)
    }
}

impl<W: Widget> SuperElement<Pod<W>, ViewCtx> for ZStackElement {
    fn upcast(ctx: &mut ViewCtx, child: Pod<W>) -> Self {
        ZStackElement(ctx.boxed_pod(child))
    }

    fn with_downcast_val<R>(
        mut this: crate::core::Mut<Self>,
        f: impl FnOnce(crate::core::Mut<Pod<W>>) -> R,
    ) -> (Self::Mut<'_>, R) {
        let ret = {
            let mut child = widget::ZStack::child_mut(&mut this.parent, this.idx)
                .expect("This is supposed to be a widget");
            let downcast = child.downcast();
            f(downcast)
        };

        (this, ret)
    }
}

// MARK: Sequence
pub trait ZStackSequence<State, Action = ()>:
    ViewSequence<State, Action, ViewCtx, ZStackElement>
{
}

impl<Seq, State, Action> ZStackSequence<State, Action> for Seq where
    Seq: ViewSequence<State, Action, ViewCtx, ZStackElement>
{
}

// MARK: Splice

pub struct StackSplice<'w> {
    idx: usize,
    element: WidgetMut<'w, widget::ZStack>,
    scratch: AppendVec<ZStackElement>,
}

impl<'w> StackSplice<'w> {
    fn new(element: WidgetMut<'w, widget::ZStack>) -> Self {
        Self {
            idx: 0,
            element,
            scratch: AppendVec::default(),
        }
    }
}

impl ElementSplice<ZStackElement> for StackSplice<'_> {
    fn with_scratch<R>(&mut self, f: impl FnOnce(&mut AppendVec<ZStackElement>) -> R) -> R {
        let ret = f(&mut self.scratch);
        for element in self.scratch.drain() {
            widget::ZStack::insert_child_pod(&mut self.element, element.0.inner);
            self.idx += 1;
        }
        ret
    }

    fn insert(&mut self, element: ZStackElement) {
        widget::ZStack::insert_child_pod(&mut self.element, element.0.inner);
        self.idx += 1;
    }

    fn mutate<R>(&mut self, f: impl FnOnce(Mut<ZStackElement>) -> R) -> R {
        let child = ZStackElementMut {
            parent: self.element.reborrow_mut(),
            idx: self.idx,
        };
        let ret = f(child);
        self.idx += 1;
        ret
    }

    fn skip(&mut self, n: usize) {
        self.idx += n;
    }

    fn delete<R>(&mut self, f: impl FnOnce(Mut<ZStackElement>) -> R) -> R {
        let ret = {
            let child = ZStackElementMut {
                parent: self.element.reborrow_mut(),
                idx: self.idx,
            };
            f(child)
        };
        widget::ZStack::remove_child(&mut self.element, self.idx);
        ret
    }
}
