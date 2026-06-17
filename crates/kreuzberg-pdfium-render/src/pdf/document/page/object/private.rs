pub(crate) mod internal {
    // We want to make the PdfPageObjectPrivate trait private while providing a blanket
    // implementation of PdfPageObjectCommon for any type T where T: PdfPageObjectPrivate.
    // Rust complains, however, that by doing so we are leaking the private trait outside
    // the crate.

    // Instead of making the PdfPageObjectPrivate trait private, we leave it public but place it
    // inside this pub(crate) module in order to prevent it from being visible outside the crate.

    use crate::bindgen::{FPDF_ANNOTATION, FPDF_DOCUMENT, FPDF_PAGE, FPDF_PAGEOBJECT, FS_MATRIX, FS_RECTF};
    use crate::bindings::PdfiumLibraryBindings;
    use crate::error::{PdfiumError, PdfiumInternalError};
    use crate::pdf::document::page::annotation::objects::PdfPageAnnotationObjects;
    use crate::pdf::document::page::object::group::PdfPageGroupObject;
    use crate::pdf::document::page::object::{
        PdfPageObject, PdfPageObjectCommon, PdfPageObjectOwnership, PdfPageObjectType,
    };
    use crate::pdf::document::page::objects::PdfPageObjects;
    use crate::pdf::document::page::objects::private::internal::PdfPageObjectsPrivate;
    use crate::pdf::document::page::{PdfPage, PdfPageContentRegenerationStrategy, PdfPageIndexCache};
    use crate::pdf::matrix::{PdfMatrix, PdfMatrixValue};
    use crate::pdf::quad_points::PdfQuadPoints;
    use crate::pdf::rect::PdfRect;
    use std::os::raw::c_double;

    /// Internal crate-specific functionality common to all [PdfPageObject] objects.
    pub(crate) trait PdfPageObjectPrivate<'a>: PdfPageObjectCommon<'a> {
        /// Returns the [PdfiumLibraryBindings] used by this [PdfPageObject].
        fn bindings(&self) -> &dyn PdfiumLibraryBindings;

        /// Returns the internal `FPDF_PAGEOBJECT` handle for this [PdfPageObject].
        fn object_handle(&self) -> FPDF_PAGEOBJECT;

        /// Returns the ownership hierarchy for this [PdfPageObject].
        fn ownership(&self) -> &PdfPageObjectOwnership;

        /// Sets the ownership hierarchy for this [PdfPageObject].
        fn set_ownership(&mut self, ownership: PdfPageObjectOwnership);

        /// Adds this [PdfPageObject] to the given [PdfPageObjects] collection.
        #[inline]
        fn add_object_to_page(&mut self, page_objects: &mut PdfPageObjects) -> Result<(), PdfiumError> {
            self.add_object_to_page_handle(page_objects.document_handle(), page_objects.page_handle())
        }

        fn add_object_to_page_handle(
            &mut self,
            document_handle: FPDF_DOCUMENT,
            page_handle: FPDF_PAGE,
        ) -> Result<(), PdfiumError> {
            self.bindings().FPDFPage_InsertObject(page_handle, self.object_handle());

            self.set_ownership(PdfPageObjectOwnership::owned_by_page(document_handle, page_handle));

            self.regenerate_content_after_mutation()
        }

        /// Removes this [PdfPageObject] from the [PdfPageObjects] collection that contains it.
        fn remove_object_from_page(&mut self) -> Result<(), PdfiumError> {
            match self.ownership() {
                PdfPageObjectOwnership::Page(ownership) => {
                    if self.bindings().is_true(
                        self.bindings()
                            .FPDFPage_RemoveObject(ownership.page_handle(), self.object_handle()),
                    ) {
                        match PdfPageIndexCache::get_content_regeneration_strategy_for_page(
                            ownership.document_handle(),
                            ownership.page_handle(),
                        ) {
                            Some(PdfPageContentRegenerationStrategy::AutomaticOnEveryChange) | None => {
                                PdfPage::regenerate_content_immut_for_handle(ownership.page_handle(), self.bindings())?;
                            }
                            _ => {}
                        }

                        self.set_ownership(PdfPageObjectOwnership::unowned());
                        self.regenerate_content_after_mutation()
                    } else {
                        Err(PdfiumError::PdfiumLibraryInternalError(PdfiumInternalError::Unknown))
                    }
                }
                _ => Err(PdfiumError::OwnershipNotAttachedToPage),
            }
        }

        /// Adds this [PdfPageObject] to the given [PdfPageAnnotationObjects] collection.
        fn add_object_to_annotation(
            &mut self,
            annotation_objects: &PdfPageAnnotationObjects,
        ) -> Result<(), PdfiumError> {
            match annotation_objects.ownership() {
                PdfPageObjectOwnership::AttachedAnnotation(ownership) => {
                    if self.bindings().is_true(
                        self.bindings()
                            .FPDFAnnot_AppendObject(ownership.annotation_handle(), self.object_handle()),
                    ) {
                        self.set_ownership(PdfPageObjectOwnership::owned_by_attached_annotation(
                            ownership.document_handle(),
                            ownership.page_handle(),
                            ownership.annotation_handle(),
                        ));
                        self.regenerate_content_after_mutation()
                    } else {
                        Err(PdfiumError::PdfiumLibraryInternalError(PdfiumInternalError::Unknown))
                    }
                }
                PdfPageObjectOwnership::UnattachedAnnotation(ownership) => {
                    if self.bindings().is_true(
                        self.bindings()
                            .FPDFAnnot_AppendObject(ownership.annotation_handle(), self.object_handle()),
                    ) {
                        self.set_ownership(PdfPageObjectOwnership::owned_by_unattached_annotation(
                            ownership.document_handle(),
                            ownership.annotation_handle(),
                        ));
                        self.regenerate_content_after_mutation()
                    } else {
                        Err(PdfiumError::PdfiumLibraryInternalError(PdfiumInternalError::Unknown))
                    }
                }
                _ => Err(PdfiumError::OwnershipNotAttachedToAnnotation),
            }
        }

        /// Removes this [PdfPageObject] from the [PdfPageAnnotationsObjects] collection that contains it.
        // We use inversion of control here so that PdfPageAnnotationsObjects doesn't need to care
        // whether the page object being removed is a single object or a group.
        fn remove_object_from_annotation(&mut self) -> Result<(), PdfiumError> {
            match self.ownership() {
                PdfPageObjectOwnership::AttachedAnnotation(ownership) => {
                    if let Some(index) = self.get_index_for_annotation(ownership.annotation_handle()) {
                        if self.bindings().is_true(
                            self.bindings()
                                .FPDFAnnot_RemoveObject(ownership.annotation_handle(), index),
                        ) {
                            match PdfPageIndexCache::get_content_regeneration_strategy_for_page(
                                ownership.document_handle(),
                                ownership.page_handle(),
                            ) {
                                Some(PdfPageContentRegenerationStrategy::AutomaticOnEveryChange) | None => {
                                    PdfPage::regenerate_content_immut_for_handle(
                                        ownership.page_handle(),
                                        self.bindings(),
                                    )?;
                                }
                                _ => {}
                            }

                            self.set_ownership(PdfPageObjectOwnership::unowned());
                            self.regenerate_content_after_mutation()
                        } else {
                            Err(PdfiumError::PdfiumLibraryInternalError(PdfiumInternalError::Unknown))
                        }
                    } else {
                        Err(PdfiumError::OwnershipNotAttachedToAnnotation)
                    }
                }
                PdfPageObjectOwnership::UnattachedAnnotation(ownership) => {
                    if let Some(index) = self.get_index_for_annotation(ownership.annotation_handle()) {
                        if self.bindings().is_true(
                            self.bindings()
                                .FPDFAnnot_RemoveObject(ownership.annotation_handle(), index),
                        ) {
                            self.set_ownership(PdfPageObjectOwnership::unowned());
                            self.regenerate_content_after_mutation()
                        } else {
                            Err(PdfiumError::PdfiumLibraryInternalError(PdfiumInternalError::Unknown))
                        }
                    } else {
                        Err(PdfiumError::OwnershipNotAttachedToAnnotation)
                    }
                }
                _ => Err(PdfiumError::OwnershipNotAttachedToAnnotation),
            }
        }

        // Perform a linear scan over the given annotation's page objects collection,
        // returning the index of this object if it exists in the collection. Matching
        // an object to its index in a page objects collection is necessary, for instance,
        // when removing objects from annotations; Pdfium only allows removing objects
        // from annotations by index.
        fn get_index_for_annotation(&self, annotation_handle: FPDF_ANNOTATION) -> Option<i32> {
            let mut result = None;

            for i in 0..self.bindings().FPDFAnnot_GetObjectCount(annotation_handle) {
                if self.object_handle() == self.bindings().FPDFAnnot_GetObject(annotation_handle, i) {
                    result = Some(i);

                    break;
                }
            }

            result
        }

        /// Internal implementation of [PdfPageObjectCommon::has_transparency()].
        fn has_transparency_impl(&self) -> bool {
            let bindings = self.bindings();

            bindings.is_true(bindings.FPDFPageObj_HasTransparency(self.object_handle()))
        }

        /// Internal implementation of [PdfPageObjectCommon::bounds()].
        fn bounds_impl(&self) -> Result<PdfQuadPoints, PdfiumError> {
            match PdfPageObjectType::from_pdfium(self.bindings().FPDFPageObj_GetType(self.object_handle()) as u32) {
                Ok(PdfPageObjectType::Text) | Ok(PdfPageObjectType::Image) => {
                    // Text and image page objects support tight fitting bounds via the
                    // FPDFPageObject_GetRotatedBounds() function.

                    let mut points = PdfQuadPoints::ZERO.as_pdfium();

                    let result = self
                        .bindings()
                        .FPDFPageObj_GetRotatedBounds(self.object_handle(), &mut points);

                    PdfQuadPoints::from_pdfium_as_result(result, points, self.bindings())
                }
                _ => {
                    // All other page objects support the FPDFPageObj_GetBounds() function.

                    let mut left = 0.0;

                    let mut bottom = 0.0;

                    let mut right = 0.0;

                    let mut top = 0.0;

                    let result = self.bindings().FPDFPageObj_GetBounds(
                        self.object_handle(),
                        &mut left,
                        &mut bottom,
                        &mut right,
                        &mut top,
                    );

                    PdfRect::from_pdfium_as_result(
                        result,
                        FS_RECTF {
                            left,
                            top,
                            right,
                            bottom,
                        },
                        self.bindings(),
                    )
                    .map(|r| r.to_quad_points())
                }
            }
        }

        /// Internal implementation of [PdfPageObjectCommon::transform()].
        #[inline]
        fn transform_impl(
            &self,
            a: PdfMatrixValue,
            b: PdfMatrixValue,
            c: PdfMatrixValue,
            d: PdfMatrixValue,
            e: PdfMatrixValue,
            f: PdfMatrixValue,
        ) -> Result<(), PdfiumError> {
            self.bindings().FPDFPageObj_Transform(
                self.object_handle(),
                a as c_double,
                b as c_double,
                c as c_double,
                d as c_double,
                e as c_double,
                f as c_double,
            );

            self.regenerate_content_after_mutation()
        }

        /// Internal implementation of [PdfPageObjectCommon::matrix()].
        fn get_matrix_impl(&self) -> Result<PdfMatrix, PdfiumError> {
            let mut matrix = FS_MATRIX {
                a: 0.0,
                b: 0.0,
                c: 0.0,
                d: 0.0,
                e: 0.0,
                f: 0.0,
            };

            if self
                .bindings()
                .is_true(self.bindings().FPDFPageObj_GetMatrix(self.object_handle(), &mut matrix))
            {
                Ok(PdfMatrix::from_pdfium(matrix))
            } else {
                Err(PdfiumError::PdfiumLibraryInternalError(PdfiumInternalError::Unknown))
            }
        }

        /// Resets the raw transformation matrix for this page object, overwriting
        /// the existing transformation matrix.
        fn reset_matrix_impl(&self, matrix: PdfMatrix) -> Result<(), PdfiumError> {
            if self.bindings().is_true(
                self.bindings()
                    .FPDFPageObj_SetMatrix(self.object_handle(), &matrix.as_pdfium()),
            ) {
                self.regenerate_content_after_mutation()
            } else {
                Err(PdfiumError::PdfiumLibraryInternalError(PdfiumInternalError::Unknown))
            }
        }

        /// Regenerate the containing page's content stream to reflect a change to the objects
        /// within the page objects container. The page's content regeneration strategy is
        /// taken into account.
        fn regenerate_content_after_mutation(&self) -> Result<(), PdfiumError> {
            let (document_handle, page_handle) = match self.ownership() {
                PdfPageObjectOwnership::Page(ownership) => {
                    (Some(ownership.document_handle()), Some(ownership.page_handle()))
                }
                PdfPageObjectOwnership::AttachedAnnotation(ownership) => {
                    (Some(ownership.document_handle()), Some(ownership.page_handle()))
                }
                _ => (None, None),
            };

            if let (Some(document_handle), Some(page_handle)) = (document_handle, page_handle) {
                if let Some(content_regeneration_strategy) =
                    PdfPageIndexCache::get_content_regeneration_strategy_for_page(document_handle, page_handle)
                {
                    if content_regeneration_strategy == PdfPageContentRegenerationStrategy::AutomaticOnEveryChange {
                        PdfPage::regenerate_content_immut_for_handle(page_handle, self.bindings())
                    } else {
                        Ok(())
                    }
                } else {
                    Err(PdfiumError::SourcePageIndexNotInCache)
                }
            } else {
                Ok(())
            }
        }

        /// Returns `true` if this [PdfPageObject] can be successfully cloned by calling its
        /// `try_clone()` function.
        fn is_copyable_impl(&self) -> bool;

        /// Attempts to clone this [PdfPageObject] by creating a new page object and copying across
        /// all the properties of this [PdfPageObject] to the new page object.
        fn try_copy_impl<'b>(
            &self,
            document_handle: FPDF_DOCUMENT,
            bindings: &'b dyn PdfiumLibraryBindings,
        ) -> Result<PdfPageObject<'b>, PdfiumError>;

        /// Copies this [PdfPageObject] object into a new [PdfPageXObjectFormObject], then adds
        /// the new form object to the page objects collection of the given [PdfPage],
        /// returning the new form object.
        fn copy_to_page_impl<'b>(&mut self, page: &mut PdfPage<'b>) -> Result<PdfPageObject<'b>, PdfiumError> {
            let mut object = PdfPageObject::from_pdfium(self.object_handle(), *self.ownership(), page.bindings());

            let (document_handle, page_handle) = match object.ownership() {
                PdfPageObjectOwnership::Page(ownership) => {
                    (Some(ownership.document_handle()), Some(ownership.page_handle()))
                }
                PdfPageObjectOwnership::AttachedAnnotation(ownership) => {
                    (Some(ownership.document_handle()), Some(ownership.page_handle()))
                }
                _ => (None, None),
            };

            if let (Some(document_handle), Some(page_handle)) = (document_handle, page_handle) {
                let mut group = PdfPageGroupObject::from_pdfium(document_handle, page_handle, page.bindings());

                group.push(&mut object)?;
                group.copy_to_page(page)
            } else {
                Err(PdfiumError::OwnershipNotAttachedToPage)
            }
        }

        /// Drops the page object by calling `FPDFPageObj_Destroy()`, freeing held memory.
        /// This will this object's `FPDF_OBJECT` handle. If the page object is attached
        /// to a page or an annotation, no action will be taken; Pdfium will manage
        /// deallocation of the object's memory automatically.
        #[inline]
        fn drop_impl(&self) {
            if !self.ownership().is_owned() {
                // Responsibility for de-allocation lies with us, not Pdfium, since
                // the object is not attached to a page or an annotation.

                self.bindings().FPDFPageObj_Destroy(self.object_handle());
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::prelude::*;
    use crate::utils::test::test_bind_to_pdfium;

    #[test]
    fn test_object_get_translation() -> Result<(), PdfiumError> {
        // Tests to make sure we can retrieve the correct horizontal and vertical translation deltas
        // from an object after applying a translation transformation.

        let pdfium = test_bind_to_pdfium();

        let mut document = pdfium.create_new_pdf()?;

        let mut page = document.pages_mut().create_page_at_start(PdfPagePaperSize::a4())?;

        let mut object = PdfPagePathObject::new_rect(
            &document,
            PdfRect::new_from_values(100.0, 100.0, 400.0, 400.0),
            Some(PdfColor::RED),
            Some(PdfPoints::new(1.0)),
            None,
        )?;

        object.translate(PdfPoints::new(250.0), PdfPoints::new(350.0))?;

        let object = page.objects_mut().add_path_object(object)?;

        assert_eq!(object.get_horizontal_translation().value, 250.0);
        assert_eq!(object.get_vertical_translation().value, 350.0);
        assert_eq!(object.get_horizontal_scale(), 1.0);
        assert_eq!(object.get_vertical_scale(), 1.0);
        assert_eq!(object.get_rotation_clockwise_degrees(), 0.0);

        Ok(())
    }

    #[test]
    fn test_object_get_scale() -> Result<(), PdfiumError> {
        // Tests to make sure we can retrieve the correct horizontal and vertical scale factors
        // from an object after applying a scale transformation.

        let pdfium = test_bind_to_pdfium();

        let mut document = pdfium.create_new_pdf()?;

        let mut page = document.pages_mut().create_page_at_start(PdfPagePaperSize::a4())?;

        let mut object = PdfPagePathObject::new_rect(
            &document,
            PdfRect::new_from_values(100.0, 100.0, 400.0, 400.0),
            Some(PdfColor::RED),
            Some(PdfPoints::new(1.0)),
            None,
        )?;

        object.scale(1.75, 2.25)?;

        let object = page.objects_mut().add_path_object(object)?;

        assert_eq!(object.get_horizontal_scale(), 1.75);
        assert_eq!(object.get_vertical_scale(), 2.25);
        assert_eq!(object.get_horizontal_translation().value, 0.0);
        assert_eq!(object.get_vertical_translation().value, 0.0);
        assert_eq!(object.get_rotation_clockwise_degrees(), 0.0);

        Ok(())
    }

    #[test]
    fn test_object_get_rotation() -> Result<(), PdfiumError> {
        // Tests to make sure we can retrieve the correct clockwise rotation angle from an object
        // after applying a rotation transformation.

        let pdfium = test_bind_to_pdfium();

        let mut document = pdfium.create_new_pdf()?;

        let mut page = document.pages_mut().create_page_at_start(PdfPagePaperSize::a4())?;

        let mut object = PdfPagePathObject::new_rect(
            &document,
            PdfRect::new_from_values(100.0, 100.0, 400.0, 400.0),
            Some(PdfColor::RED),
            Some(PdfPoints::new(1.0)),
            None,
        )?;

        object.rotate_clockwise_degrees(35.0)?;

        let object = page.objects_mut().add_path_object(object)?;

        assert_eq!(object.get_rotation_clockwise_degrees(), 35.0);
        assert_eq!(object.get_horizontal_translation().value, 0.0);
        assert_eq!(object.get_vertical_translation().value, 0.0);
        assert_eq!(object.get_horizontal_scale(), 0.819_152_06); // Rotating affects the scale factors
        assert_eq!(object.get_vertical_scale(), 0.819_152_06); // Rotating affects the scale factors

        Ok(())
    }

    #[test]
    fn test_object_get_skew() -> Result<(), PdfiumError> {
        // Tests to make sure we can retrieve the correct skew axes values from an object
        // after applying a skew transformation.

        let pdfium = test_bind_to_pdfium();

        let mut document = pdfium.create_new_pdf()?;

        let mut page = document.pages_mut().create_page_at_start(PdfPagePaperSize::a4())?;

        let mut object = PdfPagePathObject::new_rect(
            &document,
            PdfRect::new_from_values(100.0, 100.0, 400.0, 400.0),
            Some(PdfColor::RED),
            Some(PdfPoints::new(1.0)),
            None,
        )?;

        object.skew_degrees(15.5, 25.5)?;

        let object = page.objects_mut().add_path_object(object)?;

        assert_eq!((object.get_x_axis_skew_degrees() * 10.0).round() / 10.0, 15.5); // Handles the returned value being a tiny bit off, e.g. 15.4999 instead of 15.5
        assert_eq!((object.get_y_axis_skew_degrees() * 10.0).round() / 10.0, 25.5); // Handles the returned value being a tiny bit off, e.g. 25.4999 instead of 25.5
        assert_eq!(object.get_horizontal_translation().value, 0.0);
        assert_eq!(object.get_vertical_translation().value, 0.0);
        assert_eq!(object.get_horizontal_scale(), 1.0);
        assert_eq!(object.get_vertical_scale(), 1.0);
        assert_eq!(
            (object.get_rotation_counter_clockwise_degrees() * 10.0).round() / 10.0,
            15.5
        ); // Rotation angle will be the same as the x axis skew angle.

        Ok(())
    }
}
