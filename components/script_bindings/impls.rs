use std::fmt::{self, Display, Formatter};
use std::hash::Hash;
use std::marker::PhantomData;
use std::str::FromStr;
use std::sync::Arc;

use bluetooth_traits::BluetoothError;
use canvas_traits::webgl::GLContextAttributes;
use html5ever::tokenizer::{TokenSink, Tokenizer};
use html5ever::tree_builder::{Tracer as HtmlTracer, TreeBuilder, TreeSink};
use itertools::Itertools;
use js::jsapi::JSTracer;
use net_traits::request::{
    CacheMode as NetTraitsRequestCache, CredentialsMode as NetTraitsRequestCredentials,
    Destination as NetTraitsRequestDestination, Origin, RedirectMode as NetTraitsRequestRedirect,
    Referrer as NetTraitsRequestReferrer, Request as NetTraitsRequest,
    RequestMode as NetTraitsRequestMode,
};
use net_traits::ReferrerPolicy as MsgReferrerPolicy;
use script_layout_interface::{LayoutElementType, LayoutNodeType};
use script_traits::MediaSessionActionType;
use selectors::attr::{AttrSelectorOperation, CaseSensitivity, NamespaceConstraint};
use selectors::bloom::BloomFilter;
use selectors::matching::{ElementSelectorFlags, MatchingContext};
use selectors::parser::{LocalName, NonTSPseudoClass, PseudoElement};
use selectors::{Element as SelectorsElement, SelectorImpl};
use serde::Serialize;
use servo_media::audio::biquad_filter_node::{BiquadFilterNodeOptions, FilterType};
use servo_media::audio::buffer_source_node::AudioBufferSourceNodeOptions;
use servo_media::audio::channel_node::ChannelNodeOptions;
use servo_media::audio::constant_source_node::ConstantSourceNodeOptions as ServoMediaConstantSourceOptions;
use servo_media::audio::context::{LatencyCategory, ProcessingState, RealTimeAudioContextOptions};
use servo_media::audio::gain_node::GainNodeOptions;
use servo_media::audio::iir_filter_node::IIRFilterNodeOptions;
use servo_media::audio::node::{
    ChannelCountMode as ServoMediaChannelCountMode,
    ChannelInterpretation as ServoMediaChannelInterpretation,
};
use servo_media::audio::oscillator_node::{
    OscillatorNodeOptions as ServoMediaOscillatorOptions,
    OscillatorType as ServoMediaOscillatorType,
};
use servo_media::audio::panner_node::{DistanceModel, PannerNodeOptions, PanningModel};
use servo_media::audio::param::ParamRate;
use servo_media::audio::stereo_panner::StereoPannerOptions as ServoMediaStereoPannerOptions;
use servo_media::streams::device_monitor::MediaDeviceKind as ServoMediaDeviceKind;
use servo_media::webrtc::{
    DataChannelInit, DataChannelState, GatheringState, IceConnectionState, SdpType,
    SessionDescription, SignalingState,
};
use style::stylesheets::RulesMutateError;
use style::values::{AtomIdent, AtomString};
use style::Namespace;
use webgpu::ErrorFilter;
use webxr_api::{
    EntityType, EnvironmentBlendMode, Handedness, LayerInit, SessionMode, TargetRayMode,
};
use xml5ever::tokenizer::XmlTokenizer;
use xml5ever::tree_builder::{Tracer as XmlTracer, TreeSink as XmlTreeSink, XmlTreeBuilder};

use crate::codegen::Bindings::AudioBufferSourceNodeBinding::AudioBufferSourceOptions;
use crate::codegen::Bindings::AudioContextBinding::{
    AudioContextLatencyCategory, AudioContextOptions,
};
use crate::codegen::Bindings::AudioNodeBinding::{ChannelCountMode, ChannelInterpretation};
use crate::codegen::Bindings::AudioParamBinding::AutomationRate;
use crate::codegen::Bindings::BaseAudioContextBinding::AudioContextState;
use crate::codegen::Bindings::BiquadFilterNodeBinding::{BiquadFilterOptions, BiquadFilterType};
use crate::codegen::Bindings::ChannelMergerNodeBinding::ChannelMergerOptions;
use crate::codegen::Bindings::ConstantSourceNodeBinding::ConstantSourceOptions;
use crate::codegen::Bindings::EventTargetBinding::{AddEventListenerOptions, EventListenerOptions};
use crate::codegen::Bindings::FakeXRDeviceBinding::FakeXRRegionType;
use crate::codegen::Bindings::GainNodeBinding::GainOptions;
use crate::codegen::Bindings::HeadersBinding::HeadersInit;
use crate::codegen::Bindings::IIRFilterNodeBinding::IIRFilterOptions;
use crate::codegen::Bindings::MediaDeviceInfoBinding::MediaDeviceKind;
use crate::codegen::Bindings::MediaSessionBinding::MediaSessionAction;
use crate::codegen::Bindings::OscillatorNodeBinding::{OscillatorOptions, OscillatorType};
use crate::codegen::Bindings::PannerNodeBinding::{
    DistanceModelType, PannerOptions, PanningModelType,
};
use crate::codegen::Bindings::PermissionStatusBinding::PermissionName;
use crate::codegen::Bindings::RTCDataChannelBinding::{RTCDataChannelInit, RTCDataChannelState};
use crate::codegen::Bindings::RTCPeerConnectionBinding::{
    RTCIceConnectionState, RTCIceGatheringState, RTCSignalingState,
};
use crate::codegen::Bindings::RTCSessionDescriptionBinding::{
    RTCSdpType, RTCSessionDescriptionInit,
};
use crate::codegen::Bindings::RequestBinding::{
    ReferrerPolicy, RequestCache, RequestCredentials, RequestDestination, RequestInfo, RequestInit,
    RequestMethods, RequestMode, RequestRedirect,
};
use crate::codegen::Bindings::SecurityPolicyViolationEventBinding::SecurityPolicyViolationEventDisposition;
use crate::codegen::Bindings::StereoPannerNodeBinding::StereoPannerOptions;
use crate::codegen::Bindings::WebGLRenderingContextBinding::WebGLContextAttributes;
use crate::codegen::Bindings::WebGPUBinding::GPUFeatureNameValues::pairs;
use crate::codegen::Bindings::WebGPUBinding::{
    GPUCanvasConfiguration, GPUErrorFilter, GPUFeatureName, GPUTextureDescriptor,
};
use crate::codegen::Bindings::XRInputSourceBinding::{XRHandedness, XRTargetRayMode};
use crate::codegen::Bindings::XRSessionBinding::XREnvironmentBlendMode;
use crate::codegen::Bindings::XRWebGLLayerBinding::XRWebGLLayerInit;
use crate::codegen::InheritTypes::HTMLElementTypeId;
use crate::codegen::UnionTypes::{
    AddEventListenerOptionsOrBoolean, AudioContextLatencyCategoryOrDouble,
    EventListenerOptionsOrBoolean, HTMLCanvasElementOrOffscreenCanvas, StringOrUnsignedLong,
};
use crate::dom::bindings::codegen::Bindings::XRSystemBinding::XRSessionMode;
use crate::error::Error;
use crate::inheritance::{
    CharacterDataTypeId, ElementTypeId, NodeTypeId, SVGElementTypeId, SVGGraphicsElementTypeId,
};
use crate::num::Finite;
use crate::root::DomRoot;
use crate::trace::{CustomTraceable, JSTraceable};
use crate::{DomHelpers, DomTypes};

impl Clone for StringOrUnsignedLong {
    fn clone(&self) -> StringOrUnsignedLong {
        match self {
            StringOrUnsignedLong::String(s) => StringOrUnsignedLong::String(s.clone()),
            &StringOrUnsignedLong::UnsignedLong(ul) => StringOrUnsignedLong::UnsignedLong(ul),
        }
    }
}

// TODO: make all this derivables available via new Bindings.conf option
impl<T: DomTypes> Clone for GPUCanvasConfiguration<T> {
    fn clone(&self) -> Self {
        Self {
            alphaMode: self.alphaMode,
            device: self.device.clone(),
            format: self.format,
            usage: self.usage,
            viewFormats: self.viewFormats.clone(),
        }
    }
}

impl<T: DomTypes> Clone for HTMLCanvasElementOrOffscreenCanvas<T> {
    fn clone(&self) -> Self {
        match self {
            Self::HTMLCanvasElement(arg0) => Self::HTMLCanvasElement(arg0.clone()),
            Self::OffscreenCanvas(arg0) => Self::OffscreenCanvas(arg0.clone()),
        }
    }
}

impl malloc_size_of::MallocSizeOf for GPUTextureDescriptor {
    fn size_of(&self, ops: &mut malloc_size_of::MallocSizeOfOps) -> usize {
        let Self {
            parent,
            dimension,
            format,
            mipLevelCount,
            sampleCount,
            size,
            usage,
            viewFormats,
        } = self;
        parent.size_of(ops) +
            dimension.size_of(ops) +
            format.size_of(ops) +
            mipLevelCount.size_of(ops) +
            sampleCount.size_of(ops) +
            size.size_of(ops) +
            usage.size_of(ops) +
            viewFormats.size_of(ops)
    }
}

impl<T: DomTypes> malloc_size_of::MallocSizeOf for HTMLCanvasElementOrOffscreenCanvas<T> {
    fn size_of(&self, ops: &mut malloc_size_of::MallocSizeOfOps) -> usize {
        match self {
            HTMLCanvasElementOrOffscreenCanvas::HTMLCanvasElement(canvas) => canvas.size_of(ops),
            HTMLCanvasElementOrOffscreenCanvas::OffscreenCanvas(canvas) => canvas.size_of(ops),
        }
    }
}

impl From<RequestCache> for NetTraitsRequestCache {
    fn from(cache: RequestCache) -> Self {
        match cache {
            RequestCache::Default => NetTraitsRequestCache::Default,
            RequestCache::No_store => NetTraitsRequestCache::NoStore,
            RequestCache::Reload => NetTraitsRequestCache::Reload,
            RequestCache::No_cache => NetTraitsRequestCache::NoCache,
            RequestCache::Force_cache => NetTraitsRequestCache::ForceCache,
            RequestCache::Only_if_cached => NetTraitsRequestCache::OnlyIfCached,
        }
    }
}

impl From<NetTraitsRequestCache> for RequestCache {
    fn from(cache: NetTraitsRequestCache) -> Self {
        match cache {
            NetTraitsRequestCache::Default => RequestCache::Default,
            NetTraitsRequestCache::NoStore => RequestCache::No_store,
            NetTraitsRequestCache::Reload => RequestCache::Reload,
            NetTraitsRequestCache::NoCache => RequestCache::No_cache,
            NetTraitsRequestCache::ForceCache => RequestCache::Force_cache,
            NetTraitsRequestCache::OnlyIfCached => RequestCache::Only_if_cached,
        }
    }
}

impl From<RequestCredentials> for NetTraitsRequestCredentials {
    fn from(credentials: RequestCredentials) -> Self {
        match credentials {
            RequestCredentials::Omit => NetTraitsRequestCredentials::Omit,
            RequestCredentials::Same_origin => NetTraitsRequestCredentials::CredentialsSameOrigin,
            RequestCredentials::Include => NetTraitsRequestCredentials::Include,
        }
    }
}

impl From<NetTraitsRequestCredentials> for RequestCredentials {
    fn from(credentials: NetTraitsRequestCredentials) -> Self {
        match credentials {
            NetTraitsRequestCredentials::Omit => RequestCredentials::Omit,
            NetTraitsRequestCredentials::CredentialsSameOrigin => RequestCredentials::Same_origin,
            NetTraitsRequestCredentials::Include => RequestCredentials::Include,
        }
    }
}

impl From<RequestDestination> for NetTraitsRequestDestination {
    fn from(destination: RequestDestination) -> Self {
        match destination {
            RequestDestination::_empty => NetTraitsRequestDestination::None,
            RequestDestination::Audio => NetTraitsRequestDestination::Audio,
            RequestDestination::Document => NetTraitsRequestDestination::Document,
            RequestDestination::Embed => NetTraitsRequestDestination::Embed,
            RequestDestination::Font => NetTraitsRequestDestination::Font,
            RequestDestination::Frame => NetTraitsRequestDestination::Frame,
            RequestDestination::Iframe => NetTraitsRequestDestination::IFrame,
            RequestDestination::Image => NetTraitsRequestDestination::Image,
            RequestDestination::Manifest => NetTraitsRequestDestination::Manifest,
            RequestDestination::Json => NetTraitsRequestDestination::Json,
            RequestDestination::Object => NetTraitsRequestDestination::Object,
            RequestDestination::Report => NetTraitsRequestDestination::Report,
            RequestDestination::Script => NetTraitsRequestDestination::Script,
            RequestDestination::Sharedworker => NetTraitsRequestDestination::SharedWorker,
            RequestDestination::Style => NetTraitsRequestDestination::Style,
            RequestDestination::Track => NetTraitsRequestDestination::Track,
            RequestDestination::Video => NetTraitsRequestDestination::Video,
            RequestDestination::Worker => NetTraitsRequestDestination::Worker,
            RequestDestination::Xslt => NetTraitsRequestDestination::Xslt,
        }
    }
}

impl From<NetTraitsRequestDestination> for RequestDestination {
    fn from(destination: NetTraitsRequestDestination) -> Self {
        match destination {
            NetTraitsRequestDestination::None => RequestDestination::_empty,
            NetTraitsRequestDestination::Audio => RequestDestination::Audio,
            NetTraitsRequestDestination::Document => RequestDestination::Document,
            NetTraitsRequestDestination::Embed => RequestDestination::Embed,
            NetTraitsRequestDestination::Font => RequestDestination::Font,
            NetTraitsRequestDestination::Frame => RequestDestination::Frame,
            NetTraitsRequestDestination::IFrame => RequestDestination::Iframe,
            NetTraitsRequestDestination::Image => RequestDestination::Image,
            NetTraitsRequestDestination::Manifest => RequestDestination::Manifest,
            NetTraitsRequestDestination::Json => RequestDestination::Json,
            NetTraitsRequestDestination::Object => RequestDestination::Object,
            NetTraitsRequestDestination::Report => RequestDestination::Report,
            NetTraitsRequestDestination::Script => RequestDestination::Script,
            NetTraitsRequestDestination::ServiceWorker |
            NetTraitsRequestDestination::AudioWorklet |
            NetTraitsRequestDestination::PaintWorklet => {
                panic!("ServiceWorker request destination should not be exposed to DOM")
            },
            NetTraitsRequestDestination::SharedWorker => RequestDestination::Sharedworker,
            NetTraitsRequestDestination::Style => RequestDestination::Style,
            NetTraitsRequestDestination::Track => RequestDestination::Track,
            NetTraitsRequestDestination::Video => RequestDestination::Video,
            NetTraitsRequestDestination::Worker => RequestDestination::Worker,
            NetTraitsRequestDestination::Xslt => RequestDestination::Xslt,
            NetTraitsRequestDestination::WebIdentity => RequestDestination::_empty,
        }
    }
}

impl From<RequestMode> for NetTraitsRequestMode {
    fn from(mode: RequestMode) -> Self {
        match mode {
            RequestMode::Navigate => NetTraitsRequestMode::Navigate,
            RequestMode::Same_origin => NetTraitsRequestMode::SameOrigin,
            RequestMode::No_cors => NetTraitsRequestMode::NoCors,
            RequestMode::Cors => NetTraitsRequestMode::CorsMode,
        }
    }
}

impl From<NetTraitsRequestMode> for RequestMode {
    fn from(mode: NetTraitsRequestMode) -> Self {
        match mode {
            NetTraitsRequestMode::Navigate => RequestMode::Navigate,
            NetTraitsRequestMode::SameOrigin => RequestMode::Same_origin,
            NetTraitsRequestMode::NoCors => RequestMode::No_cors,
            NetTraitsRequestMode::CorsMode => RequestMode::Cors,
            NetTraitsRequestMode::WebSocket { .. } => {
                unreachable!("Websocket request mode should never be exposed to Dom")
            },
        }
    }
}

impl From<ReferrerPolicy> for MsgReferrerPolicy {
    fn from(policy: ReferrerPolicy) -> Self {
        match policy {
            ReferrerPolicy::_empty => MsgReferrerPolicy::NoReferrer,
            ReferrerPolicy::No_referrer => MsgReferrerPolicy::NoReferrer,
            ReferrerPolicy::No_referrer_when_downgrade => {
                MsgReferrerPolicy::NoReferrerWhenDowngrade
            },
            ReferrerPolicy::Origin => MsgReferrerPolicy::Origin,
            ReferrerPolicy::Origin_when_cross_origin => MsgReferrerPolicy::OriginWhenCrossOrigin,
            ReferrerPolicy::Unsafe_url => MsgReferrerPolicy::UnsafeUrl,
            ReferrerPolicy::Same_origin => MsgReferrerPolicy::SameOrigin,
            ReferrerPolicy::Strict_origin => MsgReferrerPolicy::StrictOrigin,
            ReferrerPolicy::Strict_origin_when_cross_origin => {
                MsgReferrerPolicy::StrictOriginWhenCrossOrigin
            },
        }
    }
}

impl From<MsgReferrerPolicy> for ReferrerPolicy {
    fn from(policy: MsgReferrerPolicy) -> Self {
        match policy {
            MsgReferrerPolicy::NoReferrer => ReferrerPolicy::No_referrer,
            MsgReferrerPolicy::NoReferrerWhenDowngrade => {
                ReferrerPolicy::No_referrer_when_downgrade
            },
            MsgReferrerPolicy::Origin => ReferrerPolicy::Origin,
            MsgReferrerPolicy::OriginWhenCrossOrigin => ReferrerPolicy::Origin_when_cross_origin,
            MsgReferrerPolicy::UnsafeUrl => ReferrerPolicy::Unsafe_url,
            MsgReferrerPolicy::SameOrigin => ReferrerPolicy::Same_origin,
            MsgReferrerPolicy::StrictOrigin => ReferrerPolicy::Strict_origin,
            MsgReferrerPolicy::StrictOriginWhenCrossOrigin => {
                ReferrerPolicy::Strict_origin_when_cross_origin
            },
        }
    }
}

impl From<RequestRedirect> for NetTraitsRequestRedirect {
    fn from(redirect: RequestRedirect) -> Self {
        match redirect {
            RequestRedirect::Follow => NetTraitsRequestRedirect::Follow,
            RequestRedirect::Error => NetTraitsRequestRedirect::Error,
            RequestRedirect::Manual => NetTraitsRequestRedirect::Manual,
        }
    }
}

impl From<NetTraitsRequestRedirect> for RequestRedirect {
    fn from(redirect: NetTraitsRequestRedirect) -> Self {
        match redirect {
            NetTraitsRequestRedirect::Follow => RequestRedirect::Follow,
            NetTraitsRequestRedirect::Error => RequestRedirect::Error,
            NetTraitsRequestRedirect::Manual => RequestRedirect::Manual,
        }
    }
}

impl Clone for HeadersInit {
    fn clone(&self) -> HeadersInit {
        match self {
            HeadersInit::ByteStringSequenceSequence(b) => {
                HeadersInit::ByteStringSequenceSequence(b.clone())
            },
            HeadersInit::ByteStringByteStringRecord(m) => {
                HeadersInit::ByteStringByteStringRecord(m.clone())
            },
        }
    }
}

impl Serialize for SecurityPolicyViolationEventDisposition {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        match self {
            Self::Report => serializer.serialize_str("report"),
            Self::Enforce => serializer.serialize_str("enforce"),
        }
    }
}

impl<'a, D: DomTypes> From<&'a AudioBufferSourceOptions<D>> for AudioBufferSourceNodeOptions {
    fn from(options: &'a AudioBufferSourceOptions<D>) -> Self {
        Self {
            buffer: options.buffer.as_ref().and_then(|b| {
                let b = b.as_ref()?;
                (*<D as DomHelpers<D>>::AudioBuffer_get_channels(b)).clone()
            }),
            detune: *options.detune,
            loop_enabled: options.loop_,
            loop_end: Some(*options.loopEnd),
            loop_start: Some(*options.loopStart),
            playback_rate: *options.playbackRate,
        }
    }
}

impl From<ChannelCountMode> for ServoMediaChannelCountMode {
    fn from(mode: ChannelCountMode) -> Self {
        match mode {
            ChannelCountMode::Max => ServoMediaChannelCountMode::Max,
            ChannelCountMode::Clamped_max => ServoMediaChannelCountMode::ClampedMax,
            ChannelCountMode::Explicit => ServoMediaChannelCountMode::Explicit,
        }
    }
}

impl From<ChannelInterpretation> for ServoMediaChannelInterpretation {
    fn from(interpretation: ChannelInterpretation) -> Self {
        match interpretation {
            ChannelInterpretation::Discrete => ServoMediaChannelInterpretation::Discrete,
            ChannelInterpretation::Speakers => ServoMediaChannelInterpretation::Speakers,
        }
    }
}

// https://webaudio.github.io/web-audio-api/#enumdef-automationrate
impl From<AutomationRate> for ParamRate {
    fn from(rate: AutomationRate) -> Self {
        match rate {
            AutomationRate::A_rate => ParamRate::ARate,
            AutomationRate::K_rate => ParamRate::KRate,
        }
    }
}

impl From<ProcessingState> for AudioContextState {
    fn from(state: ProcessingState) -> Self {
        match state {
            ProcessingState::Suspended => AudioContextState::Suspended,
            ProcessingState::Running => AudioContextState::Running,
            ProcessingState::Closed => AudioContextState::Closed,
        }
    }
}

impl<'a> From<&'a BiquadFilterOptions> for BiquadFilterNodeOptions {
    fn from(options: &'a BiquadFilterOptions) -> Self {
        Self {
            gain: *options.gain,
            q: *options.Q,
            frequency: *options.frequency,
            detune: *options.detune,
            filter: options.type_.into(),
        }
    }
}

impl From<BiquadFilterType> for FilterType {
    fn from(filter: BiquadFilterType) -> FilterType {
        match filter {
            BiquadFilterType::Lowpass => FilterType::LowPass,
            BiquadFilterType::Highpass => FilterType::HighPass,
            BiquadFilterType::Bandpass => FilterType::BandPass,
            BiquadFilterType::Lowshelf => FilterType::LowShelf,
            BiquadFilterType::Highshelf => FilterType::HighShelf,
            BiquadFilterType::Peaking => FilterType::Peaking,
            BiquadFilterType::Allpass => FilterType::AllPass,
            BiquadFilterType::Notch => FilterType::Notch,
        }
    }
}

impl From<BluetoothError> for Error {
    fn from(error: BluetoothError) -> Self {
        match error {
            BluetoothError::Type(message) => Error::Type(message),
            BluetoothError::Network => Error::Network,
            BluetoothError::NotFound => Error::NotFound,
            BluetoothError::NotSupported => Error::NotSupported,
            BluetoothError::Security => Error::Security,
            BluetoothError::InvalidState => Error::InvalidState,
        }
    }
}

impl<'a> From<&'a ChannelMergerOptions> for ChannelNodeOptions {
    fn from(options: &'a ChannelMergerOptions) -> Self {
        Self {
            channels: options.numberOfInputs as u8,
        }
    }
}

impl<'a> From<&'a ConstantSourceOptions> for ServoMediaConstantSourceOptions {
    fn from(options: &'a ConstantSourceOptions) -> Self {
        Self {
            offset: *options.offset,
        }
    }
}

impl From<RulesMutateError> for Error {
    fn from(other: RulesMutateError) -> Self {
        match other {
            RulesMutateError::Syntax => Error::Syntax,
            RulesMutateError::IndexSize => Error::IndexSize,
            RulesMutateError::HierarchyRequest => Error::HierarchyRequest,
            RulesMutateError::InvalidState => Error::InvalidState,
        }
    }
}

impl From<AddEventListenerOptionsOrBoolean> for AddEventListenerOptions {
    fn from(options: AddEventListenerOptionsOrBoolean) -> Self {
        match options {
            AddEventListenerOptionsOrBoolean::AddEventListenerOptions(options) => options,
            AddEventListenerOptionsOrBoolean::Boolean(capture) => Self {
                parent: EventListenerOptions { capture },
                once: false,
            },
        }
    }
}

impl From<EventListenerOptionsOrBoolean> for EventListenerOptions {
    fn from(options: EventListenerOptionsOrBoolean) -> Self {
        match options {
            EventListenerOptionsOrBoolean::EventListenerOptions(options) => options,
            EventListenerOptionsOrBoolean::Boolean(capture) => Self { capture },
        }
    }
}

impl From<FakeXRRegionType> for EntityType {
    fn from(x: FakeXRRegionType) -> Self {
        match x {
            FakeXRRegionType::Point => EntityType::Point,
            FakeXRRegionType::Plane => EntityType::Plane,
            FakeXRRegionType::Mesh => EntityType::Mesh,
        }
    }
}

impl From<XRHandedness> for Handedness {
    fn from(h: XRHandedness) -> Self {
        match h {
            XRHandedness::None => Handedness::None,
            XRHandedness::Left => Handedness::Left,
            XRHandedness::Right => Handedness::Right,
        }
    }
}

impl From<XRTargetRayMode> for TargetRayMode {
    fn from(t: XRTargetRayMode) -> Self {
        match t {
            XRTargetRayMode::Gaze => TargetRayMode::Gaze,
            XRTargetRayMode::Tracked_pointer => TargetRayMode::TrackedPointer,
            XRTargetRayMode::Screen => TargetRayMode::Screen,
            XRTargetRayMode::Transient_pointer => TargetRayMode::TransientPointer,
        }
    }
}

impl<'a> From<&'a GainOptions> for GainNodeOptions {
    fn from(options: &'a GainOptions) -> Self {
        Self {
            gain: *options.gain,
        }
    }
}

impl From<ErrorFilter> for GPUErrorFilter {
    fn from(filter: ErrorFilter) -> Self {
        match filter {
            ErrorFilter::Validation => GPUErrorFilter::Validation,
            ErrorFilter::OutOfMemory => GPUErrorFilter::Out_of_memory,
            ErrorFilter::Internal => GPUErrorFilter::Internal,
        }
    }
}

impl<'a> From<&'a WebGLContextAttributes> for GLContextAttributes {
    fn from(attrs: &'a WebGLContextAttributes) -> GLContextAttributes {
        GLContextAttributes {
            alpha: attrs.alpha,
            depth: attrs.depth,
            stencil: attrs.stencil,
            antialias: attrs.antialias,
            premultiplied_alpha: attrs.premultipliedAlpha,
            preserve_drawing_buffer: attrs.preserveDrawingBuffer,
        }
    }
}

impl<'a> From<&'a IIRFilterOptions> for IIRFilterNodeOptions {
    fn from(options: &'a IIRFilterOptions) -> Self {
        let feedforward: Vec<f64> =
            (*options.feedforward.iter().map(|v| **v).collect_vec()).to_vec();
        let feedback: Vec<f64> = (*options.feedback.iter().map(|v| **v).collect_vec()).to_vec();
        Self {
            feedforward: Arc::new(feedforward),
            feedback: Arc::new(feedback),
        }
    }
}

impl From<ServoMediaDeviceKind> for MediaDeviceKind {
    fn from(kind: ServoMediaDeviceKind) -> MediaDeviceKind {
        match kind {
            ServoMediaDeviceKind::AudioInput => MediaDeviceKind::Audioinput,
            ServoMediaDeviceKind::AudioOutput => MediaDeviceKind::Audiooutput,
            ServoMediaDeviceKind::VideoInput => MediaDeviceKind::Videoinput,
        }
    }
}

impl From<MediaSessionAction> for MediaSessionActionType {
    fn from(action: MediaSessionAction) -> MediaSessionActionType {
        match action {
            MediaSessionAction::Play => MediaSessionActionType::Play,
            MediaSessionAction::Pause => MediaSessionActionType::Pause,
            MediaSessionAction::Seekbackward => MediaSessionActionType::SeekBackward,
            MediaSessionAction::Seekforward => MediaSessionActionType::SeekForward,
            MediaSessionAction::Previoustrack => MediaSessionActionType::PreviousTrack,
            MediaSessionAction::Nexttrack => MediaSessionActionType::NextTrack,
            MediaSessionAction::Skipad => MediaSessionActionType::SkipAd,
            MediaSessionAction::Stop => MediaSessionActionType::Stop,
            MediaSessionAction::Seekto => MediaSessionActionType::SeekTo,
        }
    }
}

impl From<NodeTypeId> for LayoutNodeType {
    #[inline(always)]
    fn from(node_type: NodeTypeId) -> LayoutNodeType {
        match node_type {
            NodeTypeId::Element(e) => LayoutNodeType::Element(e.into()),
            NodeTypeId::CharacterData(CharacterDataTypeId::Text(_)) => LayoutNodeType::Text,
            x => unreachable!("Layout should not traverse nodes of type {:?}", x),
        }
    }
}

impl From<ElementTypeId> for LayoutElementType {
    #[inline(always)]
    fn from(element_type: ElementTypeId) -> LayoutElementType {
        match element_type {
            ElementTypeId::HTMLElement(HTMLElementTypeId::HTMLBodyElement) => {
                LayoutElementType::HTMLBodyElement
            },
            ElementTypeId::HTMLElement(HTMLElementTypeId::HTMLBRElement) => {
                LayoutElementType::HTMLBRElement
            },
            ElementTypeId::HTMLElement(HTMLElementTypeId::HTMLCanvasElement) => {
                LayoutElementType::HTMLCanvasElement
            },
            ElementTypeId::HTMLElement(HTMLElementTypeId::HTMLHtmlElement) => {
                LayoutElementType::HTMLHtmlElement
            },
            ElementTypeId::HTMLElement(HTMLElementTypeId::HTMLIFrameElement) => {
                LayoutElementType::HTMLIFrameElement
            },
            ElementTypeId::HTMLElement(HTMLElementTypeId::HTMLImageElement) => {
                LayoutElementType::HTMLImageElement
            },
            ElementTypeId::HTMLElement(HTMLElementTypeId::HTMLMediaElement(_)) => {
                LayoutElementType::HTMLMediaElement
            },
            ElementTypeId::HTMLElement(HTMLElementTypeId::HTMLInputElement) => {
                LayoutElementType::HTMLInputElement
            },
            ElementTypeId::HTMLElement(HTMLElementTypeId::HTMLObjectElement) => {
                LayoutElementType::HTMLObjectElement
            },
            ElementTypeId::HTMLElement(HTMLElementTypeId::HTMLParagraphElement) => {
                LayoutElementType::HTMLParagraphElement
            },
            ElementTypeId::HTMLElement(HTMLElementTypeId::HTMLTableCellElement) => {
                LayoutElementType::HTMLTableCellElement
            },
            ElementTypeId::HTMLElement(HTMLElementTypeId::HTMLTableColElement) => {
                LayoutElementType::HTMLTableColElement
            },
            ElementTypeId::HTMLElement(HTMLElementTypeId::HTMLTableElement) => {
                LayoutElementType::HTMLTableElement
            },
            ElementTypeId::HTMLElement(HTMLElementTypeId::HTMLTableRowElement) => {
                LayoutElementType::HTMLTableRowElement
            },
            ElementTypeId::HTMLElement(HTMLElementTypeId::HTMLTableSectionElement) => {
                LayoutElementType::HTMLTableSectionElement
            },
            ElementTypeId::HTMLElement(HTMLElementTypeId::HTMLTextAreaElement) => {
                LayoutElementType::HTMLTextAreaElement
            },
            ElementTypeId::SVGElement(SVGElementTypeId::SVGGraphicsElement(
                SVGGraphicsElementTypeId::SVGSVGElement,
            )) => LayoutElementType::SVGSVGElement,
            _ => LayoutElementType::Element,
        }
    }
}

impl<'a> From<&'a OscillatorOptions> for ServoMediaOscillatorOptions {
    fn from(options: &'a OscillatorOptions) -> Self {
        Self {
            oscillator_type: options.type_.into(),
            freq: *options.frequency,
            detune: *options.detune,
            periodic_wave_options: None, // XXX
        }
    }
}

impl From<OscillatorType> for ServoMediaOscillatorType {
    fn from(oscillator_type: OscillatorType) -> Self {
        match oscillator_type {
            OscillatorType::Sine => ServoMediaOscillatorType::Sine,
            OscillatorType::Square => ServoMediaOscillatorType::Square,
            OscillatorType::Sawtooth => ServoMediaOscillatorType::Sawtooth,
            OscillatorType::Triangle => ServoMediaOscillatorType::Triangle,
            OscillatorType::Custom => ServoMediaOscillatorType::Custom,
        }
    }
}

impl<'a> From<&'a PannerOptions> for PannerNodeOptions {
    fn from(options: &'a PannerOptions) -> Self {
        Self {
            panning_model: options.panningModel.into(),
            distance_model: options.distanceModel.into(),
            position_x: *options.positionX,
            position_y: *options.positionY,
            position_z: *options.positionZ,
            orientation_x: *options.orientationX,
            orientation_y: *options.orientationY,
            orientation_z: *options.orientationZ,
            ref_distance: *options.refDistance,
            max_distance: *options.maxDistance,
            rolloff_factor: *options.rolloffFactor,
            cone_inner_angle: *options.coneInnerAngle,
            cone_outer_angle: *options.coneOuterAngle,
            cone_outer_gain: *options.coneOuterGain,
        }
    }
}

impl From<DistanceModelType> for DistanceModel {
    fn from(model: DistanceModelType) -> Self {
        match model {
            DistanceModelType::Linear => DistanceModel::Linear,
            DistanceModelType::Inverse => DistanceModel::Inverse,
            DistanceModelType::Exponential => DistanceModel::Exponential,
        }
    }
}

impl From<PanningModelType> for PanningModel {
    fn from(model: PanningModelType) -> Self {
        match model {
            PanningModelType::Equalpower => PanningModel::EqualPower,
            PanningModelType::HRTF => PanningModel::HRTF,
        }
    }
}

impl From<PermissionName> for embedder_traits::PermissionName {
    fn from(permission_name: PermissionName) -> Self {
        match permission_name {
            PermissionName::Geolocation => embedder_traits::PermissionName::Geolocation,
            PermissionName::Notifications => embedder_traits::PermissionName::Notifications,
            PermissionName::Push => embedder_traits::PermissionName::Push,
            PermissionName::Midi => embedder_traits::PermissionName::Midi,
            PermissionName::Camera => embedder_traits::PermissionName::Camera,
            PermissionName::Microphone => embedder_traits::PermissionName::Microphone,
            PermissionName::Speaker => embedder_traits::PermissionName::Speaker,
            PermissionName::Device_info => embedder_traits::PermissionName::DeviceInfo,
            PermissionName::Background_sync => embedder_traits::PermissionName::BackgroundSync,
            PermissionName::Bluetooth => embedder_traits::PermissionName::Bluetooth,
            PermissionName::Persistent_storage => {
                embedder_traits::PermissionName::PersistentStorage
            },
        }
    }
}

impl From<&RTCDataChannelInit> for DataChannelInit {
    fn from(init: &RTCDataChannelInit) -> DataChannelInit {
        DataChannelInit {
            label: String::new(),
            id: init.id,
            max_packet_life_time: init.maxPacketLifeTime,
            max_retransmits: init.maxRetransmits,
            negotiated: init.negotiated,
            ordered: init.ordered,
            protocol: init.protocol.to_string(),
        }
    }
}

impl From<DataChannelState> for RTCDataChannelState {
    fn from(state: DataChannelState) -> RTCDataChannelState {
        match state {
            DataChannelState::Connecting | DataChannelState::__Unknown(_) => {
                RTCDataChannelState::Connecting
            },
            DataChannelState::Open => RTCDataChannelState::Open,
            DataChannelState::Closing => RTCDataChannelState::Closing,
            DataChannelState::Closed => RTCDataChannelState::Closed,
        }
    }
}

impl<'a> From<&'a StereoPannerOptions> for ServoMediaStereoPannerOptions {
    fn from(options: &'a StereoPannerOptions) -> Self {
        Self { pan: *options.pan }
    }
}

impl From<EnvironmentBlendMode> for XREnvironmentBlendMode {
    fn from(x: EnvironmentBlendMode) -> Self {
        match x {
            EnvironmentBlendMode::Opaque => XREnvironmentBlendMode::Opaque,
            EnvironmentBlendMode::AlphaBlend => XREnvironmentBlendMode::Alpha_blend,
            EnvironmentBlendMode::Additive => XREnvironmentBlendMode::Additive,
        }
    }
}

impl<'a> From<&'a XRWebGLLayerInit> for LayerInit {
    fn from(init: &'a XRWebGLLayerInit) -> LayerInit {
        LayerInit::WebGLLayer {
            alpha: init.alpha,
            antialias: init.antialias,
            depth: init.depth,
            stencil: init.stencil,
            framebuffer_scale_factor: *init.framebufferScaleFactor as f32,
            ignore_depth_values: init.ignoreDepthValues,
        }
    }
}

impl GPUErrorFilter {
    pub fn as_webgpu(&self) -> ErrorFilter {
        match self {
            GPUErrorFilter::Validation => ErrorFilter::Validation,
            GPUErrorFilter::Out_of_memory => ErrorFilter::OutOfMemory,
            GPUErrorFilter::Internal => ErrorFilter::Internal,
        }
    }
}

// manual hash derived
// TODO: allow derivables in bindings.conf
impl std::hash::Hash for GPUFeatureName {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        core::mem::discriminant(self).hash(state);
    }
}

impl Eq for GPUFeatureName {}

unsafe impl<Handle: JSTraceable + Clone, Sink: TreeSink<Handle = Handle> + JSTraceable>
    CustomTraceable for TreeBuilder<Handle, Sink>
{
    unsafe fn trace(&self, trc: *mut JSTracer) {
        struct Tracer<Handle>(*mut JSTracer, PhantomData<Handle>);
        let tracer = Tracer::<Handle>(trc, PhantomData);

        impl<Handle: JSTraceable> HtmlTracer for Tracer<Handle> {
            type Handle = Handle;
            #[allow(crown::unrooted_must_root)]
            fn trace_handle(&self, node: &Handle) {
                unsafe {
                    node.trace(self.0);
                }
            }
        }

        self.trace_handles(&tracer);
        self.sink.trace(trc);
    }
}

#[allow(unsafe_code)]
unsafe impl<Handle: JSTraceable + Clone, Sink: TokenSink<Handle = Handle> + CustomTraceable>
    CustomTraceable for Tokenizer<Sink>
{
    unsafe fn trace(&self, trc: *mut JSTracer) {
        let tree_builder = &self.sink;
        self.sink.trace(trc);
    }
}

#[allow(unsafe_code)]
unsafe impl<Handle: JSTraceable + Clone, Sink: JSTraceable + XmlTreeSink<Handle = Handle>>
    CustomTraceable for XmlTokenizer<XmlTreeBuilder<Handle, Sink>>
{
    unsafe fn trace(&self, trc: *mut JSTracer) {
        struct Tracer<Handle>(*mut JSTracer, PhantomData<Handle>);
        let tracer = Tracer(trc, PhantomData);

        impl<Handle: JSTraceable> XmlTracer for Tracer<Handle> {
            type Handle = Handle;
            #[allow(crown::unrooted_must_root)]
            fn trace_handle(&self, node: &Handle) {
                unsafe {
                    node.trace(self.0);
                }
            }
        }

        let tree_builder = &self.sink;
        tree_builder.trace_handles(&tracer);
        tree_builder.sink.trace(trc);
    }
}

impl Display for PermissionName {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

// this should be autogenerate by bindings
impl FromStr for GPUFeatureName {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        pairs
            .iter()
            .find(|&&(key, _)| s == key)
            .map(|&(_, ev)| ev)
            .ok_or(())
    }
}

impl From<AudioContextLatencyCategory> for LatencyCategory {
    fn from(category: AudioContextLatencyCategory) -> Self {
        match category {
            AudioContextLatencyCategory::Balanced => LatencyCategory::Balanced,
            AudioContextLatencyCategory::Interactive => LatencyCategory::Interactive,
            AudioContextLatencyCategory::Playback => LatencyCategory::Playback,
        }
    }
}

impl<'a> From<&'a AudioContextOptions> for RealTimeAudioContextOptions {
    fn from(options: &AudioContextOptions) -> Self {
        RealTimeAudioContextOptions {
            sample_rate: *options.sampleRate.unwrap_or(Finite::wrap(44100.)),
            latency_hint: match options.latencyHint {
                AudioContextLatencyCategoryOrDouble::AudioContextLatencyCategory(category) => {
                    category.into()
                },
                AudioContextLatencyCategoryOrDouble::Double(_) => LatencyCategory::Interactive, // TODO
            },
        }
    }
}

impl From<SessionDescription> for RTCSessionDescriptionInit {
    fn from(desc: SessionDescription) -> Self {
        let type_ = match desc.type_ {
            SdpType::Answer => RTCSdpType::Answer,
            SdpType::Offer => RTCSdpType::Offer,
            SdpType::Pranswer => RTCSdpType::Pranswer,
            SdpType::Rollback => RTCSdpType::Rollback,
        };
        RTCSessionDescriptionInit {
            type_,
            sdp: desc.sdp.into(),
        }
        .into()
    }
}

impl<'a> From<&'a RTCSessionDescriptionInit> for SessionDescription {
    fn from(desc: &'a RTCSessionDescriptionInit) -> Self {
        let type_ = match desc.type_ {
            RTCSdpType::Answer => SdpType::Answer,
            RTCSdpType::Offer => SdpType::Offer,
            RTCSdpType::Pranswer => SdpType::Pranswer,
            RTCSdpType::Rollback => SdpType::Rollback,
        };
        SessionDescription {
            type_,
            sdp: desc.sdp.to_string(),
        }
    }
}

impl From<GatheringState> for RTCIceGatheringState {
    fn from(state: GatheringState) -> Self {
        match state {
            GatheringState::New => RTCIceGatheringState::New,
            GatheringState::Gathering => RTCIceGatheringState::Gathering,
            GatheringState::Complete => RTCIceGatheringState::Complete,
        }
    }
}

impl From<IceConnectionState> for RTCIceConnectionState {
    fn from(state: IceConnectionState) -> Self {
        match state {
            IceConnectionState::New => RTCIceConnectionState::New,
            IceConnectionState::Checking => RTCIceConnectionState::Checking,
            IceConnectionState::Connected => RTCIceConnectionState::Connected,
            IceConnectionState::Completed => RTCIceConnectionState::Completed,
            IceConnectionState::Disconnected => RTCIceConnectionState::Disconnected,
            IceConnectionState::Failed => RTCIceConnectionState::Failed,
            IceConnectionState::Closed => RTCIceConnectionState::Closed,
        }
    }
}

impl From<SignalingState> for RTCSignalingState {
    fn from(state: SignalingState) -> Self {
        match state {
            SignalingState::Stable => RTCSignalingState::Stable,
            SignalingState::HaveLocalOffer => RTCSignalingState::Have_local_offer,
            SignalingState::HaveRemoteOffer => RTCSignalingState::Have_remote_offer,
            SignalingState::HaveLocalPranswer => RTCSignalingState::Have_local_pranswer,
            SignalingState::HaveRemotePranswer => RTCSignalingState::Have_remote_pranswer,
            SignalingState::Closed => RTCSignalingState::Closed,
        }
    }
}

impl From<XRSessionMode> for SessionMode {
    fn from(mode: XRSessionMode) -> SessionMode {
        match mode {
            XRSessionMode::Immersive_vr => SessionMode::ImmersiveVR,
            XRSessionMode::Immersive_ar => SessionMode::ImmersiveAR,
            XRSessionMode::Inline => SessionMode::Inline,
        }
    }
}

/*impl<T: SelectorsElement + crate::DomObject + Into<DomRoot<T>>> SelectorsElement for DomRoot<T> {
    type Impl = T::Impl;

    fn opaque(&self) -> ::selectors::OpaqueElement {
        T::opaque(&*self)
    }

    fn parent_element(&self) -> Option<DomRoot<T>> {
        T::parent_element(&*self).map(Into::into)
    }

    fn parent_node_is_shadow_root(&self) -> bool {
        T::parent_node_is_shadow_root(&*self)
    }

    fn containing_shadow_host(&self) -> Option<DomRoot<T>> {
        T::containing_shadow_host(&*self).map(Into::into)
    }

    fn is_pseudo_element(&self) -> bool {
        T::is_pseudo_element(&*self)
    }

    fn match_pseudo_element(
        &self,
        pseudo: &<<T as selectors::Element>::Impl as selectors::SelectorImpl>::PseudoElement,
        context: &mut MatchingContext<Self::Impl>,
    ) -> bool {
        T::match_pseudo_element(&*self, pseudo, context)
    }

    fn prev_sibling_element(&self) -> Option<DomRoot<T>> {
        T::prev_sibling_element(&*self).map(Into::into)
    }

    fn next_sibling_element(&self) -> Option<DomRoot<T>> {
        T::next_sibling_element(&*self).map(Into::into)
    }

    fn first_element_child(&self) -> Option<DomRoot<T>> {
        T::first_element_child(&*self).map(Into::into)
    }

    fn attr_matches(
        &self,
        ns: &NamespaceConstraint<&<<T as selectors::Element>::Impl as selectors::SelectorImpl>::NamespaceUrl>,
        local_name: &<<T as SelectorsElement>::Impl as selectors::SelectorImpl>::LocalName,
        operation: &AttrSelectorOperation<&<<T as selectors::Element>::Impl as selectors::SelectorImpl>::AttrValue>,
    ) -> bool {
        T::attr_matches(&*self, ns, local_name, operation)
    }

    fn is_root(&self) -> bool {
        T::is_root(&*self)
    }

    fn is_empty(&self) -> bool {
        T::is_empty(&*self)
    }

    fn has_local_name(&self, local_name: &<<T as SelectorsElement>::Impl as selectors::SelectorImpl>::BorrowedLocalName) -> bool {
        T::has_local_name(&*self, local_name)
    }

    fn has_namespace(&self, ns: &<<T as selectors::Element>::Impl as selectors::SelectorImpl>::BorrowedNamespaceUrl) -> bool {
        T::has_namespace(&*self, ns)
    }

    fn is_same_type(&self, other: &Self) -> bool {
        T::is_same_type(&*self, &*other)
    }

    fn match_non_ts_pseudo_class(
        &self,
        pseudo_class: &<<T as selectors::Element>::Impl as selectors::SelectorImpl>::NonTSPseudoClass,
        ctx: &mut MatchingContext<Self::Impl>,
    ) -> bool {
        T::match_non_ts_pseudo_class(&*self, pseudo_class, ctx)
    }

    fn is_link(&self) -> bool {
        T::is_link(&*self)
    }

    fn has_id(&self, id: &<<T as selectors::Element>::Impl as selectors::SelectorImpl>::Identifier, case_sensitivity: CaseSensitivity) -> bool {
        T::has_id(&*self, id, case_sensitivity)
    }

    fn is_part(&self, name: &<<T as selectors::Element>::Impl as selectors::SelectorImpl>::Identifier) -> bool {
        T::is_part(&*self, name)
    }

    fn imported_part(&self, ident: &<<T as selectors::Element>::Impl as selectors::SelectorImpl>::Identifier) -> Option<<<T as selectors::Element>::Impl as selectors::SelectorImpl>::Identifier> {
        T::imported_part(&*self, ident)
    }

    fn has_class(&self, name: &<<T as selectors::Element>::Impl as selectors::SelectorImpl>::Identifier, case_sensitivity: CaseSensitivity) -> bool {
        T::has_class(&*self, name, case_sensitivity)
    }

    fn is_html_element_in_html_document(&self) -> bool {
        T::is_html_element_in_html_document(&*self)
    }

    fn is_html_slot_element(&self) -> bool {
        T::is_html_slot_element(&*self)
    }

    fn apply_selector_flags(&self, flags: ElementSelectorFlags) {
        T::apply_selector_flags(&*self, flags)
    }

    fn add_element_unique_hashes(&self, filter: &mut BloomFilter) -> bool {
        T::add_element_unique_hashes(&*self, filter)
    }

    fn has_custom_state(&self, name: &<<T as selectors::Element>::Impl as selectors::SelectorImpl>::Identifier) -> bool {
        T::has_custom_state(&*self, name)
    }
}*/

impl<T: fmt::Debug + crate::DomObject> fmt::Debug for DomRoot<T> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        T::fmt(&*self, f)
    }
}
