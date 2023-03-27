/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/. */

use std::cell::Cell;
use std::collections::VecDeque;
use std::rc::Rc;

use crate::dom::bindings::codegen::Bindings::ReadableStreamDefaultControllerBinding::ReadableStreamDefaultControllerMethods;
use crate::dom::bindings::reflector::{reflect_dom_object, DomObject, Reflector};
use crate::dom::promisenativehandler::{Callback, PromiseNativeHandler};
use crate::realms::{enter_realm, InRealm};
use crate::script_runtime::JSContext;
use crate::task::TaskBox;
use dom_struct::dom_struct;
use js::jsapi::{Heap, JSObject};
use js::jsval::{JSVal, UndefinedValue};
use js::rust::{Handle, HandleValue, MutableHandleValue};

use super::bindings::callback::ExceptionHandling;
use super::bindings::codegen::Bindings::QueuingStrategyBinding::QueuingStrategySize;
use super::bindings::codegen::Bindings::ReadableStreamBinding::{
    ReadableStreamController, UnderlyingSource,
};
use super::bindings::error::Fallible;
use super::promise::Promise;
use super::readablestream::ReadableStream;
use super::types::GlobalScope;
use crate::dom::bindings::cell::DomRefCell;
use crate::dom::bindings::root::{Dom, DomRoot, MutNullableDom};

pub trait UnderlyingSourceAlgorithmsBase {
    fn StartCallback(
        &self,
        cx: JSContext,
        controller: &ReadableStreamController,
        retval: MutableHandleValue,
    ) -> Fallible<()>;

    /// A promise-returning algorithm that pulls data from the underlying byte
    /// source
    fn PullCallback(
        &self,
        cx: JSContext,
        controller: &ReadableStreamController,
    ) -> Fallible<Rc<Promise>>;

    /// A promise-returning algorithm, taking one argument (the cancel reason),
    /// which communicates a requested cancelation to the underlying byte source
    fn CancelCallback(&self, cx: JSContext, reason: Option<HandleValue>) -> Fallible<Rc<Promise>>;

    /// Implement this when you need to release underlying resources immediately
    /// from closed(canceled)/errored streams, without waiting for GC.
    fn ReleaseObjects();

    // Fetch wants to special-case BodyStream-based streams
    //virtual BodyStreamHolder* GetBodyStreamHolder() { return nullptr; }

    // https://streams.spec.whatwg.org/#other-specs-rs-create
    /// By "native" we mean "instances initialized via the above set up or set up
    /// with byte reading support algorithms (not, e.g., on web-developer-created
    /// instances)"
    fn is_native(&self) -> bool;

    //protected:
    //virtual ~UnderlyingSourceAlgorithmsBase() = default;
}

#[dom_struct]
pub struct ReadableStreamDefaultController {
    reflector_: Reflector,
    /// All algoritems packed together:
    /// - Close algoritm: A promise-returning algorithm, taking one argument (the cancel reason), which communicates a requested cancelation to the underlying source
    /// - Pull algoritm: A promise-returning algorithm that pulls data from the underlying source
    algorithms: UnderlyingSourceAlgorithms,
    /// A boolean flag indicating whether the stream has been closed by its underlying source, but still has chunks in its internal queue that have not yet been read
    closeRequested: Cell<bool>,
    /// A boolean flag set to true if the stream’s mechanisms requested a call to the underlying source's pull algorithm to pull more data, but the pull could not yet be done since a previous call is still executing
    pullAgain: Cell<bool>,
    /// A boolean flag set to true while the underlying source's pull algorithm is executing and the returned promise has not yet fulfilled, used to prevent reentrant calls
    pulling: Cell<bool>,
    /// A list representing the stream’s internal queue of chunks
    #[ignore_malloc_size_of = "Defined in rust-mozjs"]
    queue: DomRefCell<VecDeque<Heap<JSVal>>>,
    /// The total size of all the chunks stored in queue (see § 8.1 Queue-with-sizes)
    queueTotalSize: Cell<f64>,
    /// A boolean flag indicating whether the underlying source has finished starting
    started: Cell<bool>,
    /// A number supplied to the constructor as part of the stream’s queuing strategy, indicating the point at which the stream will apply backpressure to its underlying source
    strategyHWM: Cell<f64>,
    /// An algorithm to calculate the size of enqueued chunks, as part of the stream’s queuing strategy
    ///
    /// If missing use default value (1) per https://streams.spec.whatwg.org/#make-size-algorithm-from-size-function
    #[ignore_malloc_size_of = "Rc is hard"]
    strategySizeAlgorithm: Option<Rc<QueuingStrategySize>>,
    /// The ReadableStream instance controlled
    stream: MutNullableDom<ReadableStream>,
}

impl ReadableStreamDefaultControllerMethods for ReadableStreamDefaultController {
    fn GetDesiredSize(&self) -> Option<f64> {
        todo!()
    }

    fn Close(&self) -> Fallible<()> {
        todo!()
    }

    fn Enqueue(&self, cx: JSContext, chunk: HandleValue) -> Fallible<()> {
        todo!()
    }

    fn Error(&self, cx: JSContext, e: HandleValue) -> Fallible<()> {
        todo!()
    }
}

impl ReadableStreamDefaultController {
    fn new_inherited() -> Self {
        Self {
            reflector_: Reflector::new(),
            algorithms: todo!(),
            closeRequested: todo!(),
            pullAgain: todo!(),
            pulling: todo!(),
            queue: todo!(),
            queueTotalSize: todo!(),
            started: todo!(),
            strategyHWM: todo!(),
            strategySizeAlgorithm: todo!(),
            stream: Default::default(),
        }
    }

    fn new(global: &GlobalScope) -> DomRoot<Self> {
        reflect_dom_object(Box::new(Self::new_inherited()), global)
    }
}

#[derive(JSTraceable, MallocSizeOf)]
pub struct UnderlyingSourceAlgorithms {
    #[ignore_malloc_size_of = "idk bindings so unclear ownership semantics"]
    under: UnderlyingSource,
    //global: GlobalScope,
}

impl UnderlyingSourceAlgorithms {
    fn new(global: &GlobalScope, underlying_source_dict: UnderlyingSource) -> Self {
        Self {
            under: underlying_source_dict,
            //global: GlobalScope:,
        }
    }
}

impl UnderlyingSourceAlgorithmsBase for UnderlyingSourceAlgorithms {
    fn StartCallback(
        &self,
        cx: JSContext,
        controller: &ReadableStreamController,
        retval: MutableHandleValue,
    ) -> Fallible<()> {
        rooted!(in(*cx) let mut val = UndefinedValue());
        if let Some(callback) = self.under.start {
            val.set(callback.Call__(controller, ExceptionHandling::Rethrow)?);
        }

        retval.set(val.get());
        Ok(())
    }

    fn PullCallback(
        &self,
        cx: JSContext,
        controller: &ReadableStreamController,
    ) -> Fallible<Rc<Promise>> {
        if let Some(callback) = self.under.pull {
            return callback.Call__(controller, ExceptionHandling::Rethrow);
        } else {
            // It is optional so return primise with undefined
            Promise::new_resolved_to_undefined(
                &GlobalScope::current().expect("No current global"),
                cx,
            )
        }
    }

    fn CancelCallback(&self, cx: JSContext, reason: Option<HandleValue>) -> Fallible<Rc<Promise>> {
        if let Some(callback) = self.under.cancel {
            return callback.Call__(reason, ExceptionHandling::Rethrow);
        } else {
            // It is optional so return primise with undefined
            Promise::new_resolved_to_undefined(
                &GlobalScope::current().expect("No current global"),
                cx,
            )
        }
    }

    fn ReleaseObjects() {
        todo!()
    }

    fn is_native(&self) -> bool {
        true
    }
}

// https://streams.spec.whatwg.org/#set-up-readable-stream-default-controller-from-underlying-source
pub fn setup_readable_stream_default_controller_from_underlying_source(
    cx: JSContext,
    stream: DomRoot<ReadableStream>,
    underlying_source_obj: Handle<*mut JSObject>,
    underlying_source_dict: UnderlyingSource,
    highwatermark: f64,
    size_algorithm: Option<Rc<QueuingStrategySize>>,
) -> Fallible<()> {
    // Step 1.
    let controller = ReadableStreamDefaultController::new(&*stream.global());

    // Step 2 - 7
    let algorithms: UnderlyingSourceAlgorithms =
        UnderlyingSourceAlgorithms::new(&*stream.global(), underlying_source_dict);

    // Step 8:
    SetUpReadableStreamDefaultController(
        cx,
        stream,
        controller,
        algorithms,
        highwatermark,
        size_algorithm,
    )
}

// https://streams.spec.whatwg.org/#set-up-readable-stream-default-controller
fn SetUpReadableStreamDefaultController(
    cx: JSContext,
    stream: DomRoot<ReadableStream>,
    controller: DomRoot<ReadableStreamDefaultController>,
    algorithms: UnderlyingSourceAlgorithms,
    highwatermark: f64,
    size_algorithm: Option<Rc<QueuingStrategySize>>,
) -> Fallible<()> {
    // Step 1.
    assert!(stream.Controller().is_some());

    // Step 2.
    controller.stream.or_init(|| stream);

    // Step 3.
    // TODO(streams): Abstract over different controllers and use ResetQueue(controller);
    controller.queue.borrow_mut().clear();
    controller.queueTotalSize.set(0.0);

    // Step 4.
    controller.started.set(false);
    controller.closeRequested.set(false);
    controller.pullAgain.set(false);
    controller.pulling.set(false);

    // Step 5.
    controller.strategySizeAlgorithm = size_algorithm;
    controller.strategyHWM.set(highwatermark);

    // Step 6.
    // Step 7.
    controller.algorithms = algorithms;

    // Step 8.
    stream.SetController(controller);

    // Step 9. Default algorithm returns undefined. See Step 2 of
    // https://streams.spec.whatwg.org/#set-up-readable-stream-default-controller
    rooted!(in(*cx) let mut start_result = UndefinedValue());
    //RefPtr<ReadableStreamDefaultController> ccontroller = controller;
    algorithms.StartCallback(cx, &controller, start_result.handle_mut())?;

    // Step 10.
    let global = &*stream.global();
    let realm = enter_realm(&*global);
    let comp = InRealm::Entered(&realm);
    let start_promise: Rc<Promise> = Promise::new_in_current_realm(global, comp);
    start_promise.resolve(cx, start_result.handle());
    // Step 11 & 12
    #[derive(JSTraceable, MallocSizeOf)]
    struct Handler {
        controller: DomRoot<ReadableStreamDefaultController>,
    }

    impl Handler {
        pub fn new(controller: DomRoot<ReadableStreamDefaultController>) -> Box<dyn Callback> {
            Box::new(Self {
                controller,
            })
        }
    }

    impl Callback for Handler {
        fn callback(&self, cx: JSContext, _v: HandleValue, _realm: InRealm) {
            // Step 11.1
          self.controller.started.set(true);

          // Step 11.2
          self.controller.pulling.set(false);

          // Step 11.3
          self.controller.pullAgain.set(false);

          // Step 11.4:
          ReadableStreamDefaultControllerCallPullIfNeeded(cx, self.controller).unwrap();
        }
    }
    let handler = PromiseNativeHandler::new(
        &global,
        Some(Handler::new(controller)),
        Some(todo!()),
    );
    start_promise.append_native_handler(&handler, comp);
    /*
    // Step 11 & 12:
    startPromise->AddCallbacksWithCycleCollectedArgs(
        [](JSContext* aCx, JS::Handle<JS::Value> aValue, ErrorResult& aRv,
           ReadableStreamDefaultController* controller)
            MOZ_CAN_RUN_SCRIPT_BOUNDARY {
              MOZ_ASSERT(controller);

              // Step 11.1
              controller->SetStarted(true);

              // Step 11.2
              controller->SetPulling(false);

              // Step 11.3
              controller->SetPullAgain(false);

              // Step 11.4:
              ReadableStreamDefaultControllerCallPullIfNeeded(
                  aCx, MOZ_KnownLive(controller), aRv);
            },

        [](JSContext* aCx, JS::Handle<JS::Value> aValue, ErrorResult& aRv,
           ReadableStreamDefaultController* controller) {
          // Step 12.1
          ReadableStreamDefaultControllerError(aCx, controller, aValue, aRv);
        },
        RefPtr(controller));*/
}

// https://streams.spec.whatwg.org/#readable-stream-default-controller-call-pull-if-needed
fn ReadableStreamDefaultControllerCallPullIfNeeded(
    cx: JSContext,
    controller: &ReadableStreamDefaultController,
) -> Fallible<()> {
    // Step 1.
    let should_pull = ReadableStreamDefaultControllerShouldCallPull(controller);

    // Step 2.
    if !should_pull {
        return Ok(());
    }
    /*
    // Step 3.
    if (aController->Pulling()) {
      // Step 3.1
      aController->SetPullAgain(true);
      // Step 3.2
      return;
    }

    // Step 4.
    MOZ_ASSERT(!aController->PullAgain());

    // Step 5.
    aController->SetPulling(true);

    // Step 6.
    RefPtr<UnderlyingSourceAlgorithmsBase> algorithms =
        aController->GetAlgorithms();
    RefPtr<Promise> pullPromise =
        algorithms->PullCallback(aCx, *aController, aRv);
    if (aRv.Failed()) {
      return;
    }

    // Step 7 + 8:
    pullPromise->AddCallbacksWithCycleCollectedArgs(
        [](JSContext* aCx, JS::Handle<JS::Value> aValue, ErrorResult& aRv,
           ReadableStreamDefaultController* mController)
            MOZ_CAN_RUN_SCRIPT_BOUNDARY {
              // Step 7.1
              mController->SetPulling(false);
              // Step 7.2
              if (mController->PullAgain()) {
                // Step 7.2.1
                mController->SetPullAgain(false);

                // Step 7.2.2
                ErrorResult rv;
                ReadableStreamDefaultControllerCallPullIfNeeded(
                    aCx, MOZ_KnownLive(mController), aRv);
              }
            },
        [](JSContext* aCx, JS::Handle<JS::Value> aValue, ErrorResult& aRv,
           ReadableStreamDefaultController* mController) {
          // Step 8.1
          ReadableStreamDefaultControllerError(aCx, mController, aValue, aRv);
        },
        RefPtr(aController));*/
    Ok(())
}
