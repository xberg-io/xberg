// A macro that creates transformation functions. The created functions require the containing
// impl block to contain a private transform_impl() function. Both the specification of self and the
// data type of the return value of transform_impl() can be passed as parameters into this macro.
// This offers more flexibility than defining a trait; for instance, this macro can create functions
// that operate on either &mut self or self, whereas a trait cannot.
#[doc(hidden)]
#[macro_export]
macro_rules! create_transform_setters {
    // Notes on the macro parameters specified below:

    // $self_:ty - the type of self taken by each function, e.g. &self, Self, &mut self, ...
    // $ret_:ty - The return value for each function, e.g. Self, Result<Self, ...>, (), Result<(), ...>
    // This must match the return value of the private transform_impl() function.
    // $doc_ref_:literal - The wording used to refer to the containing impl block, with no trailing punctuation.
    // $doc_ref_period_:literal - The wording used to refer to the containing impl block, with a trailing period.
    // $doc_ref_comma_:literal - The wording used to refer to the containing impl block, with a trailing comma.
    // $custom_doc_:literal - Any custom documentation to include at the end of each function's doc comment.
    // $reset_matrix_visibility_:ident -  An identifier indicating whether the reset_matrix() and
    // reset_matrix_to_identity() functions created by this macro should be public or private.
    // Not all transformable objects allow setting the transformation matrix directly; PdfPage is an
    // example. For these objects, the set_matrix() function should be private.
    (
        $self_:ty,
        $ret_:ty,
        $doc_ref_:literal,
        $doc_ref_period_:literal,
        $doc_ref_comma_:literal,
        $custom_doc_:literal,
        $reset_matrix_visibility_:vis
    ) => {
        /// Applies the given transformation, expressed as six values representing the six configurable
        /// elements of a nine-element 3x3 PDF transformation matrix, to
        #[doc = $doc_ref_period_ ]
        ///
        #[doc = $custom_doc_ ]
        ///
        /// To move, scale, rotate, or skew
        #[doc = $doc_ref_comma_ ]
        /// consider using one or more of
        /// the following functions. Internally they all use [Self::transform], but are
        /// probably easier to use (and certainly clearer in their intent) in most situations.
        ///
        /// * [Self::translate]: changes the position of
        #[doc = $doc_ref_period_ ]
        /// * [Self::scale]: changes the size of
        #[doc = $doc_ref_period_ ]
        /// * [Self::flip_horizontally]: flips
        #[doc = $doc_ref_ ]
        /// horizontally around its origin.
        /// * [Self::flip_vertically]: flips
        #[doc = $doc_ref_ ]
        /// vertically around its origin.
        /// * [Self::rotate_clockwise_degrees], [Self::rotate_counter_clockwise_degrees],
        /// [Self::rotate_clockwise_radians], [Self::rotate_counter_clockwise_radians]: rotates
        #[doc = $doc_ref_ ]
        /// around its origin.
        /// * [Self::skew_degrees], [Self::skew_radians]: skews
        #[doc = $doc_ref_ ]
        /// relative to its axes.
        ///
        /// **The order in which transformations are applied is significant.**
        /// For example, the result of rotating _then_ translating an object may be vastly different
        /// from translating _then_ rotating the same object.
        ///
        /// An overview of PDF transformation matrices can be found in the PDF Reference Manual
        /// version 1.7 on page 204; a detailed description can be found in section 4.2.3 on page 207.
        #[inline]
        pub fn transform(
            self: $self_,
            a: PdfMatrixValue,
            b: PdfMatrixValue,
            c: PdfMatrixValue,
            d: PdfMatrixValue,
            e: PdfMatrixValue,
            f: PdfMatrixValue,
        ) -> $ret_ {
            self.transform_impl(a, b, c, d, e, f)
        }

        /// Applies the given transformation, expressed as a [PdfMatrix], to
        #[doc = $doc_ref_period_ ]
        ///
        #[doc = $custom_doc_ ]
        #[inline]
        pub fn apply_matrix(self: $self_, matrix: PdfMatrix) -> $ret_ {
            self.transform_impl(
                matrix.a(),
                matrix.b(),
                matrix.c(),
                matrix.d(),
                matrix.e(),
                matrix.f(),
            )
        }

        // TODO: AJRC - 29/7/22 - remove deprecated set_matrix() function in 0.9.0
        // as part of tracking issue https://github.com/ajrcarey/pdfium-render/issues/36
        #[deprecated(
            since = "0.8.15",
            note = "This function has been renamed to better reflect its behaviour. Use the apply_matrix() function instead."
        )]
        #[doc(hidden)]
        #[inline]
        pub fn set_matrix(self: $self_, matrix: PdfMatrix) -> $ret_ {
            self.apply_matrix(matrix)
        }

        /// Resets the transform matrix for
        #[doc = $doc_ref_ ]
        /// to the the given [PdfMatrix], overriding any previously applied transformations.
        ///
        #[doc = $custom_doc_ ]
        #[inline]
        #[allow(dead_code)]
        $reset_matrix_visibility_ fn reset_matrix(self: $self_, matrix: PdfMatrix) -> $ret_ {
            self.reset_matrix_impl(matrix)
        }

        /// Resets the transformation matrix for
        #[doc = $doc_ref_ ]
        /// to the identity matrix, undoing any previously applied transformations.
        ///
        #[doc = $custom_doc_ ]
        #[inline]
        #[allow(dead_code)]
        $reset_matrix_visibility_ fn reset_matrix_to_identity(self: $self_) -> $ret_ {
            self.reset_matrix(PdfMatrix::IDENTITY)
        }

        /// Moves the origin of
        #[doc = $doc_ref_ ]
        /// by the given horizontal and vertical delta distances.
        ///
        #[doc = $custom_doc_ ]
        #[inline]
        pub fn translate(self: $self_, delta_x: PdfPoints, delta_y: PdfPoints) -> $ret_ {
            self.transform(1.0, 0.0, 0.0, 1.0, delta_x.value, delta_y.value)
        }

        /// Changes the size of
        #[doc = $doc_ref_comma_ ]
        /// scaling it by the given horizontal and vertical scale factors.
        ///
        #[doc = $custom_doc_ ]
        #[inline]
        pub fn scale(
            self: $self_,
            horizontal_scale_factor: PdfMatrixValue,
            vertical_scale_factor: PdfMatrixValue,
        ) -> $ret_ {
            self.transform(
                horizontal_scale_factor,
                0.0,
                0.0,
                vertical_scale_factor,
                0.0,
                0.0,
            )
        }

        /// Flips
        #[doc = $doc_ref_ ]
        /// horizontally around its origin by applying a horizontal scale factor of -1.
        ///
        #[doc = $custom_doc_ ]
        #[inline]
        pub fn flip_horizontally(self: $self_) -> $ret_ {
            self.scale(-1.0, 1.0)
        }

        /// Flips
        #[doc = $doc_ref_ ]
        /// vertically around its origin by applying a vertical scale factor of -1.
        ///
        #[doc = $custom_doc_ ]
        #[inline]
        pub fn flip_vertically(self: $self_) -> $ret_ {
            self.scale(1.0, -1.0)
        }

        /// Reflects
        #[doc = $doc_ref_ ]
        /// by flipping it both horizontally and vertically around its origin.
        ///
        #[doc = $custom_doc_ ]
        #[inline]
        pub fn reflect(self: $self_) -> $ret_ {
            self.scale(-1.0, -1.0)
        }

        /// Rotates
        #[doc = $doc_ref_ ]
        /// counter-clockwise by the given number of degrees.
        ///
        #[doc = $custom_doc_ ]
        #[inline]
        pub fn rotate_counter_clockwise_degrees(self: $self_, degrees: PdfMatrixValue) -> $ret_ {
            self.rotate_counter_clockwise_radians(degrees.to_radians())
        }

        /// Rotates
        #[doc = $doc_ref_ ]
        /// clockwise by the given number of degrees.
        ///
        #[doc = $custom_doc_ ]
        #[inline]
        pub fn rotate_clockwise_degrees(self: $self_, degrees: PdfMatrixValue) -> $ret_ {
            self.rotate_counter_clockwise_degrees(-degrees)
        }

        /// Rotates
        #[doc = $doc_ref_ ]
        /// counter-clockwise by the given number of radians.
        ///
        #[doc = $custom_doc_ ]
        #[inline]
        pub fn rotate_counter_clockwise_radians(self: $self_, radians: PdfMatrixValue) -> $ret_ {
            let cos_theta = radians.cos();

            let sin_theta = radians.sin();

            self.transform(cos_theta, sin_theta, -sin_theta, cos_theta, 0.0, 0.0)
        }

        /// Rotates
        #[doc = $doc_ref_ ]
        /// clockwise by the given number of radians.
        ///
        #[doc = $custom_doc_ ]
        #[inline]
        pub fn rotate_clockwise_radians(self: $self_, radians: PdfMatrixValue) -> $ret_ {
            self.rotate_counter_clockwise_radians(-radians)
        }

        /// Skews the axes of
        #[doc = $doc_ref_ ]
        /// by the given angles in degrees.
        ///
        #[doc = $custom_doc_ ]
        #[inline]
        pub fn skew_degrees(
            self: $self_,
            x_axis_skew: PdfMatrixValue,
            y_axis_skew: PdfMatrixValue,
        ) -> $ret_ {
            self.skew_radians(x_axis_skew.to_radians(), y_axis_skew.to_radians())
        }

        /// Skews the axes of
        #[doc = $doc_ref_ ]
        /// by the given angles in radians.
        ///
        #[doc = $custom_doc_ ]
        #[inline]
        pub fn skew_radians(
            self: $self_,
            x_axis_skew: PdfMatrixValue,
            y_axis_skew: PdfMatrixValue,
        ) -> $ret_ {
            self.transform(1.0, x_axis_skew.tan(), y_axis_skew.tan(), 1.0, 0.0, 0.0)
        }
    };
    ($self_:ty, $ret_:ty, $doc_ref_:literal, $doc_ref_period_:literal, $doc_ref_comma_:literal, $custom_doc_:literal) => {
        create_transform_setters!(
            $self_,
            $ret_,
            $doc_ref_,
            $doc_ref_period_,
            $doc_ref_comma_,
            $custom_doc_,
            pub // Make the set_matrix() function public by default.
        );
    };
    ($self_:ty, $ret_:ty, $doc_ref_:literal, $doc_ref_period_:literal, $doc_ref_comma_:literal) => {
        create_transform_setters!(
            $self_,
            $ret_,
            $doc_ref_,
            $doc_ref_period_,
            $doc_ref_comma_,
            "",  // No custom documentation for this set of setter functions.
            pub  // Make the set_matrix() function public by default.
        );
    };
}

// A macro that creates functions that read the current transformation matrix. The created functions
// require the containing impl block to contain a private get_matrix_impl() -> Result<PdfMatrix, PdfiumError>
// function. This could be implemented as a trait, but for the sake of consistency with the
// create_transform_setters!() macro (which could _not_ be implemented as a trait), we stick with
// using a macro.
#[doc(hidden)]
#[macro_export]
macro_rules! create_transform_getters {
    ($doc_ref_:literal, $doc_ref_period_:literal, $doc_ref_comma_:literal) => {
        /// Returns the transformation matrix currently applied to
        #[doc = $doc_ref_period_ ]
        #[inline]
        pub fn matrix(&self) -> Result<PdfMatrix, PdfiumError> {
            self.get_matrix_impl()
        }

        /// Returns the current horizontal and vertical translation of the origin of
        #[doc = $doc_ref_period_ ]
        #[inline]
        pub fn get_translation(&self) -> (PdfPoints, PdfPoints) {
            (self.get_horizontal_translation(), self.get_vertical_translation())
        }

        /// Returns the current horizontal translation of the origin of
        #[doc = $doc_ref_period_ ]
        #[inline]
        pub fn get_horizontal_translation(&self) -> PdfPoints {
            self.matrix()
                .map(|matrix| PdfPoints::new(matrix.e()))
                .unwrap_or(PdfPoints::ZERO)
        }

        /// Returns the current vertical translation of the origin of
        #[doc = $doc_ref_period_ ]
        #[inline]
        pub fn get_vertical_translation(&self) -> PdfPoints {
            self.matrix()
                .map(|matrix| PdfPoints::new(matrix.f()))
                .unwrap_or(PdfPoints::ZERO)
        }

        /// Returns the current horizontal and vertical scale factors applied to
        #[doc = $doc_ref_period_ ]
        #[inline]
        pub fn get_scale(&self) -> (PdfMatrixValue, PdfMatrixValue) {
            (self.get_horizontal_scale(), self.get_vertical_scale())
        }

        /// Returns the current horizontal scale factor applied to
        #[doc = $doc_ref_period_ ]
        #[inline]
        pub fn get_horizontal_scale(&self) -> PdfMatrixValue {
            self.matrix().map(|matrix| matrix.a()).unwrap_or(0.0)
        }

        /// Returns the current vertical scale factor applied to
        #[doc = $doc_ref_period_ ]
        #[inline]
        pub fn get_vertical_scale(&self) -> PdfMatrixValue {
            self.matrix().map(|matrix| matrix.d()).unwrap_or(0.0)
        }

        /// Returns the counter-clockwise rotation applied to
        #[doc = $doc_ref_comma_ ]
        /// in degrees.
        ///
        /// If the object is both rotated and skewed, the return value of this function will reflect
        /// the combined operation.
        #[inline]
        pub fn get_rotation_counter_clockwise_degrees(&self) -> PdfMatrixValue {
            self.get_rotation_counter_clockwise_radians().to_degrees()
        }

        /// Returns the clockwise rotation applied to
        #[doc = $doc_ref_comma_ ]
        /// in degrees.
        ///
        /// If the object is both rotated and skewed, the return value of this function will reflect
        /// the combined operation.
        #[inline]
        pub fn get_rotation_clockwise_degrees(&self) -> PdfMatrixValue {
            -self.get_rotation_counter_clockwise_degrees()
        }

        /// Returns the counter-clockwise rotation applied to
        #[doc = $doc_ref_comma_ ]
        /// in radians.
        ///
        /// If the object is both rotated and skewed, the return value of this function will reflect
        /// the combined operation.
        #[inline]
        pub fn get_rotation_counter_clockwise_radians(&self) -> PdfMatrixValue {
            self.matrix()
                .map(|matrix| matrix.b().atan2(matrix.a()))
                .unwrap_or(0.0)
        }

        /// Returns the clockwise rotation applied to
        #[doc = $doc_ref_comma_ ]
        /// in radians.
        ///
        /// If the object is both rotated and skewed, the return value of this function will reflect
        /// the combined operation.
        #[inline]
        pub fn get_rotation_clockwise_radians(&self) -> PdfMatrixValue {
            -self.get_rotation_counter_clockwise_radians()
        }

        /// Returns the current x axis and y axis skew angles applied to
        #[doc = $doc_ref_comma_ ]
        /// in degrees.
        ///
        /// If the object is both rotated and skewed, the return value of this function will reflect
        /// the combined operation.
        #[inline]
        pub fn get_skew_degrees(&self) -> (PdfMatrixValue, PdfMatrixValue) {
            (self.get_x_axis_skew_degrees(), self.get_y_axis_skew_degrees())
        }

        /// Returns the current x axis skew angle applied to
        #[doc = $doc_ref_comma_ ]
        /// in degrees.
        ///
        /// If the object is both rotated and skewed, the return value of this function will reflect
        /// the combined operation.
        #[inline]
        pub fn get_x_axis_skew_degrees(&self) -> PdfMatrixValue {
            self.get_x_axis_skew_radians().to_degrees()
        }

        /// Returns the current y axis skew applied to
        #[doc = $doc_ref_comma_ ]
        /// in degrees.
        ///
        /// If the object is both rotated and skewed, the return value of this function will reflect
        /// the combined operation.
        #[inline]
        pub fn get_y_axis_skew_degrees(&self) -> PdfMatrixValue {
            self.get_y_axis_skew_radians().to_degrees()
        }

        /// Returns the current x axis and y axis skew angles applied to
        #[doc = $doc_ref_comma_ ]
        /// in radians.
        ///
        /// If the object is both rotated and skewed, the return value of this function will reflect
        /// the combined operation.
        #[inline]
        pub fn get_skew_radians(&self) -> (PdfMatrixValue, PdfMatrixValue) {
            (self.get_x_axis_skew_radians(), self.get_y_axis_skew_radians())
        }

        /// Returns the current x axis skew applied to
        #[doc = $doc_ref_comma_ ]
        /// in radians.
        ///
        /// If the object is both rotated and skewed, the return value of this function will reflect
        /// the combined operation.
        #[inline]
        pub fn get_x_axis_skew_radians(&self) -> PdfMatrixValue {
            self.matrix().map(|matrix| matrix.b().atan()).unwrap_or(0.0)
        }

        /// Returns the current y axis skew applied to
        #[doc = $doc_ref_comma_ ]
        /// in radians.
        ///
        /// If the object is both rotated and skewed, the return value of this function will reflect
        /// the combined operation.
        #[inline]
        pub fn get_y_axis_skew_radians(&self) -> PdfMatrixValue {
            self.matrix().map(|matrix| matrix.c().atan()).unwrap_or(0.0)
        }
    };
    () => {
        create_transform_getters!("this object", "this object.", "this object,");
    };
}
