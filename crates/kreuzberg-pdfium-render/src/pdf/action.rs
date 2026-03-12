//! Defines the [PdfAction] struct, exposing functionality related to a single action
//! associated with a clickable link or document bookmark.

pub mod embedded_destination;
pub mod launch;
pub mod local_destination;
pub(crate) mod private; // Keep private so that the PdfActionPrivate trait is not exposed.
pub mod remote_destination;
pub mod unsupported;
pub mod uri;

use crate::bindgen::{
    FPDF_ACTION, FPDF_DOCUMENT, PDFACTION_EMBEDDEDGOTO, PDFACTION_GOTO, PDFACTION_LAUNCH, PDFACTION_REMOTEGOTO,
    PDFACTION_UNSUPPORTED, PDFACTION_URI,
};
use crate::bindings::PdfiumLibraryBindings;
use crate::error::PdfiumError;
use crate::pdf::action::embedded_destination::PdfActionEmbeddedDestination;
use crate::pdf::action::launch::PdfActionLaunch;
use crate::pdf::action::local_destination::PdfActionLocalDestination;
use crate::pdf::action::private::internal::PdfActionPrivate;
use crate::pdf::action::remote_destination::PdfActionRemoteDestination;
use crate::pdf::action::unsupported::PdfActionUnsupported;
use crate::pdf::action::uri::PdfActionUri;

/// The type of action associated with a clickable link or document bookmark.
#[derive(Debug, Copy, Clone, PartialOrd, PartialEq)]
pub enum PdfActionType {
    GoToDestinationInSameDocument = PDFACTION_GOTO as isize,
    GoToDestinationInRemoteDocument = PDFACTION_REMOTEGOTO as isize,
    GoToDestinationInEmbeddedDocument = PDFACTION_EMBEDDEDGOTO as isize,
    Launch = PDFACTION_LAUNCH as isize,
    Uri = PDFACTION_URI as isize,
    Unsupported = PDFACTION_UNSUPPORTED as isize,
}

impl PdfActionType {
    pub(crate) fn from_pdfium(action_type: u32) -> Result<PdfActionType, PdfiumError> {
        match action_type {
            PDFACTION_GOTO => Ok(PdfActionType::GoToDestinationInSameDocument),
            PDFACTION_REMOTEGOTO => Ok(PdfActionType::GoToDestinationInRemoteDocument),
            PDFACTION_EMBEDDEDGOTO => Ok(PdfActionType::GoToDestinationInEmbeddedDocument),
            PDFACTION_LAUNCH => Ok(PdfActionType::Launch),
            PDFACTION_URI => Ok(PdfActionType::Uri),
            PDFACTION_UNSUPPORTED => Ok(PdfActionType::Unsupported),
            _ => Err(PdfiumError::UnknownActionType),
        }
    }
}

/// The action associated with a clickable link or document bookmark.
pub enum PdfAction<'a> {
    LocalDestination(PdfActionLocalDestination<'a>),
    RemoteDestination(PdfActionRemoteDestination<'a>),
    EmbeddedDestination(PdfActionEmbeddedDestination<'a>),
    Launch(PdfActionLaunch<'a>),
    Uri(PdfActionUri<'a>),
    Unsupported(PdfActionUnsupported<'a>),
}

impl<'a> PdfAction<'a> {
    pub(crate) fn from_pdfium(
        handle: FPDF_ACTION,
        document: FPDF_DOCUMENT,
        bindings: &'a dyn PdfiumLibraryBindings,
    ) -> Self {
        #[allow(clippy::unnecessary_cast)]
        match PdfActionType::from_pdfium(bindings.FPDFAction_GetType(handle) as u32)
            .unwrap_or(PdfActionType::Unsupported)
        {
            PdfActionType::Unsupported => PdfAction::Unsupported(PdfActionUnsupported::from_pdfium(handle, bindings)),
            PdfActionType::GoToDestinationInSameDocument => {
                PdfAction::LocalDestination(PdfActionLocalDestination::from_pdfium(handle, document, bindings))
            }
            PdfActionType::GoToDestinationInRemoteDocument => {
                PdfAction::RemoteDestination(PdfActionRemoteDestination::from_pdfium(handle, bindings))
            }
            PdfActionType::GoToDestinationInEmbeddedDocument => {
                PdfAction::EmbeddedDestination(PdfActionEmbeddedDestination::from_pdfium(handle, bindings))
            }
            PdfActionType::Launch => PdfAction::Launch(PdfActionLaunch::from_pdfium(handle, bindings)),
            PdfActionType::Uri => PdfAction::Uri(PdfActionUri::from_pdfium(handle, document, bindings)),
        }
    }

    #[inline]
    pub(crate) fn unwrap_as_trait(&self) -> &dyn PdfActionPrivate<'a> {
        match self {
            PdfAction::LocalDestination(action) => action,
            PdfAction::RemoteDestination(action) => action,
            PdfAction::EmbeddedDestination(action) => action,
            PdfAction::Launch(action) => action,
            PdfAction::Uri(action) => action,
            PdfAction::Unsupported(action) => action,
        }
    }

    #[inline]
    #[allow(dead_code)] // AJRC - 18/2/23 - We expect this function to be used in future.
    pub(crate) fn unwrap_as_trait_mut(&mut self) -> &mut dyn PdfActionPrivate<'a> {
        match self {
            PdfAction::LocalDestination(action) => action,
            PdfAction::RemoteDestination(action) => action,
            PdfAction::EmbeddedDestination(action) => action,
            PdfAction::Launch(action) => action,
            PdfAction::Uri(action) => action,
            PdfAction::Unsupported(action) => action,
        }
    }

    /// Returns the [PdfActionType] for this [PdfAction].
    ///
    /// Note that Pdfium does not support or recognize all PDF action types. For instance,
    /// Pdfium does not currently support or recognize the interactive Javascript action type
    /// supported by Adobe Acrobat or Foxit's commercial PDF SDK. In these cases,
    /// Pdfium will return [PdfActionType::Unsupported].
    #[inline]
    pub fn action_type(&self) -> PdfActionType {
        match self {
            PdfAction::LocalDestination(_) => PdfActionType::GoToDestinationInSameDocument,
            PdfAction::RemoteDestination(_) => PdfActionType::GoToDestinationInRemoteDocument,
            PdfAction::EmbeddedDestination(_) => PdfActionType::GoToDestinationInEmbeddedDocument,
            PdfAction::Launch(_) => PdfActionType::Launch,
            PdfAction::Uri(_) => PdfActionType::Uri,
            PdfAction::Unsupported(_) => PdfActionType::Unsupported,
        }
    }

    /// Returns `true` if this [PdfAction] has an action type other than [PdfActionType::Unsupported].
    ///
    /// The [PdfAction::as_local_destination_action()], [PdfAction::as_remote_destination_action()],
    /// [PdfAction::as_embedded_destination_action()], [PdfAction::as_launch_action()],
    /// and [PdfAction::as_uri_action()] functions can be used to access properties and functions
    /// pertaining to a specific action type.
    #[inline]
    pub fn is_supported(&self) -> bool {
        !self.is_unsupported()
    }

    /// Returns `true` if this [PdfAction] has an action type of [PdfActionType::Unsupported].
    ///
    /// Common properties shared by all [PdfAction] types can still be accessed for
    /// actions not recognized by Pdfium, but action-specific functionality will be unavailable.
    #[inline]
    pub fn is_unsupported(&self) -> bool {
        self.action_type() == PdfActionType::Unsupported
    }

    /// Returns an immutable reference to the underlying [PdfActionLocalDestination] for this [PdfAction],
    /// if this action has an action type of [PdfActionType::GoToDestinationInSameDocument].
    #[inline]
    pub fn as_local_destination_action(&self) -> Option<&PdfActionLocalDestination<'_>> {
        match self {
            PdfAction::LocalDestination(action) => Some(action),
            _ => None,
        }
    }

    /// Returns a mutable reference to the underlying [PdfActionLocalDestination] for this [PdfAction],
    /// if this action has an action type of [PdfActionType::GoToDestinationInSameDocument].
    #[inline]
    pub fn as_local_destination_action_mut(&mut self) -> Option<&mut PdfActionLocalDestination<'a>> {
        match self {
            PdfAction::LocalDestination(action) => Some(action),
            _ => None,
        }
    }

    /// Returns an immutable reference to the underlying [PdfActionRemoteDestination] for this [PdfAction],
    /// if this action has an action type of [PdfActionType::GoToDestinationInRemoteDocument].
    #[inline]
    pub fn as_remote_destination_action(&self) -> Option<&PdfActionRemoteDestination<'_>> {
        match self {
            PdfAction::RemoteDestination(action) => Some(action),
            _ => None,
        }
    }

    /// Returns a mutable reference to the underlying [PdfActionRemoteDestination] for this [PdfAction],
    /// if this action has an action type of [PdfActionType::GoToDestinationInRemoteDocument].
    #[inline]
    pub fn as_remote_destination_action_mut(&mut self) -> Option<&mut PdfActionRemoteDestination<'a>> {
        match self {
            PdfAction::RemoteDestination(action) => Some(action),
            _ => None,
        }
    }

    /// Returns an immutable reference to the underlying [PdfActionEmbeddedDestination] for this [PdfAction],
    /// if this action has an action type of [PdfActionType::GoToDestinationInEmbeddedDocument].
    #[inline]
    pub fn as_embedded_destination_action(&self) -> Option<&PdfActionEmbeddedDestination<'_>> {
        match self {
            PdfAction::EmbeddedDestination(action) => Some(action),
            _ => None,
        }
    }

    /// Returns a mutable reference to the underlying [PdfActionEmbeddedDestination] for this [PdfAction],
    /// if this action has an action type of [PdfActionType::GoToDestinationInEmbeddedDocument].
    #[inline]
    pub fn as_embedded_destination_action_mut(&mut self) -> Option<&mut PdfActionEmbeddedDestination<'a>> {
        match self {
            PdfAction::EmbeddedDestination(action) => Some(action),
            _ => None,
        }
    }

    /// Returns an immutable reference to the underlying [PdfActionLaunch] for this [PdfAction],
    /// if this action has an action type of [PdfActionType::Launch].
    #[inline]
    pub fn as_launch_action(&self) -> Option<&PdfActionLaunch<'_>> {
        match self {
            PdfAction::Launch(action) => Some(action),
            _ => None,
        }
    }

    /// Returns a mutable reference to the underlying [PdfActionLaunch] for this [PdfAction],
    /// if this action has an action type of [PdfActionType::Launch].
    #[inline]
    pub fn as_launch_action_mut(&mut self) -> Option<&mut PdfActionLaunch<'a>> {
        match self {
            PdfAction::Launch(action) => Some(action),
            _ => None,
        }
    }

    /// Returns an immutable reference to the underlying [PdfActionUri] for this [PdfAction],
    /// if this action has an action type of [PdfActionType::Uri].
    #[inline]
    pub fn as_uri_action(&self) -> Option<&PdfActionUri<'_>> {
        match self {
            PdfAction::Uri(action) => Some(action),
            _ => None,
        }
    }

    /// Returns a mutable reference to the underlying [PdfActionUri] for this [PdfAction],
    /// if this action has an action type of [PdfActionType::Uri].
    #[inline]
    pub fn as_uri_action_mut(&mut self) -> Option<&mut PdfActionUri<'a>> {
        match self {
            PdfAction::Uri(action) => Some(action),
            _ => None,
        }
    }
}

/// Functionality common to all [PdfAction] objects, regardless of their [PdfActionType].
pub trait PdfActionCommon<'a> {
    // TODO: AJRC - 18/2/23 - this trait is reserved for future expansion.
}

// Blanket implementation for all PdfAction types.

impl<'a, T> PdfActionCommon<'a> for T where T: PdfActionPrivate<'a> {}

impl<'a> PdfActionPrivate<'a> for PdfAction<'a> {
    #[inline]
    fn handle(&self) -> &FPDF_ACTION {
        self.unwrap_as_trait().handle()
    }

    #[inline]
    fn bindings(&self) -> &dyn PdfiumLibraryBindings {
        self.unwrap_as_trait().bindings()
    }
}

impl<'a> From<PdfActionLocalDestination<'a>> for PdfAction<'a> {
    #[inline]
    fn from(action: PdfActionLocalDestination<'a>) -> Self {
        Self::LocalDestination(action)
    }
}

impl<'a> From<PdfActionRemoteDestination<'a>> for PdfAction<'a> {
    #[inline]
    fn from(action: PdfActionRemoteDestination<'a>) -> Self {
        Self::RemoteDestination(action)
    }
}

impl<'a> From<PdfActionEmbeddedDestination<'a>> for PdfAction<'a> {
    #[inline]
    fn from(action: PdfActionEmbeddedDestination<'a>) -> Self {
        Self::EmbeddedDestination(action)
    }
}

impl<'a> From<PdfActionLaunch<'a>> for PdfAction<'a> {
    #[inline]
    fn from(action: PdfActionLaunch<'a>) -> Self {
        Self::Launch(action)
    }
}

impl<'a> From<PdfActionUri<'a>> for PdfAction<'a> {
    #[inline]
    fn from(action: PdfActionUri<'a>) -> Self {
        Self::Uri(action)
    }
}

impl<'a> From<PdfActionUnsupported<'a>> for PdfAction<'a> {
    #[inline]
    fn from(action: PdfActionUnsupported<'a>) -> Self {
        Self::Unsupported(action)
    }
}
