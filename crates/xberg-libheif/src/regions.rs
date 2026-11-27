use crate::lh;
use std::ptr;

use std::rc::Rc;

use crate::context::ContextOwner;
use crate::utils::get_non_null_ptr;
use crate::{HeifError, Image, ItemId, Result};

/// A native region together with a share of the context it was parsed from.
///
/// libheif resolves the transformed getters against the image properties of that context, so a
/// region keeps the context alive and, through the context's shared owner, keeps a new image
/// from being read under it until the region is dropped.
struct OwnedRegion<'a> {
    ptr: ptr::NonNull<lh::heif_region>,
    _context: Rc<ContextOwner<'a>>,
}

impl<'a> OwnedRegion<'a> {
    fn new(ptr: ptr::NonNull<lh::heif_region>, context: &Rc<ContextOwner<'a>>) -> Self {
        Self {
            ptr,
            _context: Rc::clone(context),
        }
    }

    fn as_ptr(&self) -> *mut lh::heif_region {
        self.ptr.as_ptr()
    }
}

pub struct PointRegion<'a>(OwnedRegion<'a>);

#[allow(unsafe_code)]
impl PointRegion<'_> {
    /// Get the values for a point region.
    ///
    /// This returns the coordinates in the reference coordinate space
    /// (from the parent region item).
    #[allow(unsafe_code)]
    pub fn point(&self) -> (i32, i32) {
        let mut x = 0;
        let mut y = 0;
        // SAFETY: `self.0` is the owned native region, live for the `&self` borrow; only the two out
        // coordinates are written.
        unsafe {
            lh::heif_region_get_point(self.0.as_ptr(), &mut x, &mut y);
        }
        (x, y)
    }

    /// Get the transformed values for a point region.
    ///
    /// This returns the coordinates in pixels after all transformative
    /// properties have been applied.
    #[allow(unsafe_code)]
    pub fn point_transformed(&self, image_id: ItemId) -> (f64, f64) {
        let mut x = 0.;
        let mut y = 0.;
        // SAFETY: `self.0` owns the region and shares the context whose image properties this transformed
        // getter resolves against, so both stay live; only the out coordinates are written.
        unsafe {
            lh::heif_region_get_point_transformed(self.0.as_ptr(), image_id, &mut x, &mut y);
        }
        (x, y)
    }
}

pub struct RectangleRegion<'a>(OwnedRegion<'a>);

/// Rectangle geometry.
///
/// The region is represented by a left top corner position,
/// and a size defined by width and height.
/// All the interior points and the edge are part of the region.
#[derive(Default, Copy, Clone, PartialEq, Eq)]
pub struct Rectangle {
    pub left: i32,
    pub top: i32,
    pub width: u32,
    pub height: u32,
}

#[derive(Default, Copy, Clone, PartialEq)]
pub struct RectangleTransformed {
    pub left: f64,
    pub top: f64,
    pub width: f64,
    pub height: f64,
}

#[allow(unsafe_code)]
impl RectangleRegion<'_> {
    /// Get the values for a rectangle region.
    ///
    /// This returns the values in the reference coordinate space
    /// (from the parent region item).
    #[allow(unsafe_code)]
    pub fn rectangle(&self) -> Rectangle {
        let mut res = Rectangle::default();
        // SAFETY: `self.0` is the owned native region, live for the `&self` borrow; libheif only writes the
        // four out fields of the initialized `Rectangle`.
        unsafe {
            lh::heif_region_get_rectangle(
                self.0.as_ptr(),
                &mut res.left,
                &mut res.top,
                &mut res.width,
                &mut res.height,
            );
        }
        res
    }

    /// Get the transformed values for a rectangle region.
    ///
    /// This returns the coordinates in pixels after all transformative
    /// properties have been applied.
    #[allow(unsafe_code)]
    pub fn rectangle_transformed(&self, image_id: ItemId) -> RectangleTransformed {
        let mut res = RectangleTransformed::default();
        // SAFETY: `self.0` owns the region and shares the context whose image properties this transformed
        // getter resolves against, so both stay live; only the out fields are written.
        unsafe {
            lh::heif_region_get_rectangle_transformed(
                self.0.as_ptr(),
                image_id,
                &mut res.left,
                &mut res.top,
                &mut res.width,
                &mut res.height,
            );
        }
        res
    }
}

pub struct EllipseRegion<'a>(OwnedRegion<'a>);

#[derive(Default, Copy, Clone, PartialEq, Eq)]
pub struct Ellipse {
    pub x: i32,
    pub y: i32,
    pub x_radius: u32,
    pub y_radius: u32,
}

#[derive(Default, Copy, Clone, PartialEq)]
pub struct EllipseTransformed {
    pub x: f64,
    pub y: f64,
    pub x_radius: f64,
    pub y_radius: f64,
}

#[allow(unsafe_code)]
impl EllipseRegion<'_> {
    /// Get the values for an ellipse region.
    ///
    /// This returns the values in the reference coordinate space (from
    /// the parent region item).
    /// The ellipse is represented by a center position, and a size defined
    /// by radii in the X and Y directions. All the interior points and
    /// the edge are part of the region.
    #[allow(unsafe_code)]
    pub fn ellipse(&self) -> Ellipse {
        let mut res = Ellipse::default();
        // SAFETY: `self.0` is the owned native region, live for the `&self` borrow; libheif only writes the
        // four out fields of the initialized `Ellipse`.
        unsafe {
            lh::heif_region_get_ellipse(
                self.0.as_ptr(),
                &mut res.x,
                &mut res.y,
                &mut res.x_radius,
                &mut res.y_radius,
            );
        }
        res
    }

    /// Get the transformed values for an ellipse region.
    ///
    /// This returns the coordinates in pixels after all transformative
    /// properties have been applied.
    /// The ellipse is represented by a center position, and a size defined
    /// by radii in the X and Y directions. All the interior points and
    /// the edge are part of the region.
    #[allow(unsafe_code)]
    pub fn ellipse_transformed(&self, image_id: ItemId) -> EllipseTransformed {
        let mut res = EllipseTransformed::default();
        // SAFETY: `self.0` owns the region and shares the context whose image properties this transformed
        // getter resolves against, so both stay live; only the out fields are written.
        unsafe {
            lh::heif_region_get_ellipse_transformed(
                self.0.as_ptr(),
                image_id,
                &mut res.x,
                &mut res.y,
                &mut res.x_radius,
                &mut res.y_radius,
            );
        }
        res
    }
}

pub struct PolygonRegion<'a>(OwnedRegion<'a>);

#[allow(unsafe_code)]
impl PolygonRegion<'_> {
    /// Get the points in a polygon region.
    ///
    /// This returns the values in the reference coordinate space (from
    /// the parent region item).
    ///
    /// A polygon is a sequence of points that form a closed shape.
    /// The first point does not need to be repeated as the last point.
    /// All the interior points and the edge are part of the region.
    /// The points are returned as pairs of (X, Y) coordinates.
    #[allow(unsafe_code)]
    pub fn polygon_points(&self) -> Vec<(i32, i32)> {
        // SAFETY: `self.0` is the owned native region, live for the `&self` borrow; the query only reads the
        // stored point count.
        let num_points = unsafe { lh::heif_region_get_polygon_num_points(self.0.as_ptr()) };
        let size = num_points.max(0) as usize;
        let mut points: Vec<(i32, i32)> = Vec::with_capacity(size);
        if size > 0 {
            // SAFETY: The vector reserves the point count just queried from the same immutable region, the
            // bound libheif is given, and `set_len` runs only for the points that call wrote.
            unsafe {
                lh::heif_region_get_polygon_points(self.0.as_ptr(), points.as_mut_ptr() as _);
                points.set_len(size);
            }
        }
        points
    }

    /// Get the transformed points in a polygon region.
    ///
    /// This returns the coordinates in pixels after all transformative
    /// properties have been applied.
    ///
    /// A polygon is a sequence of points that form a closed shape.
    /// The first point does not need to be repeated as the last point.
    /// All the interior points and the edge are part of the region.
    /// The points are returned as pairs of (X, Y) coordinates.
    #[allow(unsafe_code)]
    pub fn polygon_points_transformed(&self, image_id: ItemId) -> Vec<(f64, f64)> {
        // SAFETY: `self.0` is the owned native region, live for the `&self` borrow; the query only reads the
        // stored point count.
        let num_points = unsafe { lh::heif_region_get_polygon_num_points(self.0.as_ptr()) };
        let size = num_points.max(0) as usize;
        let mut points: Vec<(f64, f64)> = Vec::with_capacity(size);
        if size > 0 {
            // SAFETY: The vector reserves the point count just queried from the same immutable region, the
            // context the transform reads is kept alive by `self.0`, and `set_len` covers only the
            // points that call wrote.
            unsafe {
                lh::heif_region_get_polygon_points_transformed(self.0.as_ptr(), image_id, points.as_mut_ptr() as _);
                points.set_len(size);
            }
        }
        points
    }
}

pub struct PolylineRegion<'a>(OwnedRegion<'a>);

#[allow(unsafe_code)]
impl PolylineRegion<'_> {
    /// Get the points in a polyline region.
    ///
    /// This returns the values in the reference coordinate space (from
    /// the parent region item).
    ///
    /// A polyline is a sequence of points that does not form a closed shape.
    /// Even if the polyline is closed, the only points that are part of
    /// the region are those that intersect (even minimally) a one-pixel line
    /// drawn along the polyline.
    /// The points are returned as pairs of (X, Y) coordinates.
    #[allow(unsafe_code)]
    pub fn polyline_points(&self) -> Vec<(i32, i32)> {
        // SAFETY: `self.0` is the owned native region, live for the `&self` borrow; the query only reads the
        // stored point count.
        let num_points = unsafe { lh::heif_region_get_polyline_num_points(self.0.as_ptr()) };
        let size = num_points.max(0) as usize;
        let mut points: Vec<(i32, i32)> = Vec::with_capacity(size);
        if size > 0 {
            // SAFETY: The vector reserves the point count just queried from the same immutable region, the
            // bound libheif is given, and `set_len` runs only for the points that call wrote.
            unsafe {
                lh::heif_region_get_polyline_points(self.0.as_ptr(), points.as_mut_ptr() as _);
                points.set_len(size);
            }
        }
        points
    }

    /// Get the transformed points in a polyline region.
    ///
    /// This returns the coordinates in pixels after all transformative
    /// properties have been applied.
    ///
    /// A polyline is a sequence of points that does not form a closed shape.
    /// Even if the polyline is closed, the only points that are part of the
    /// region are those that intersect (even minimally) a one-pixel line
    /// drawn along the polyline.
    /// The points are returned as pairs of (X, Y) coordinates.
    #[allow(unsafe_code)]
    pub fn polyline_points_transformed(&self, image_id: ItemId) -> Vec<(f64, f64)> {
        // SAFETY: `self.0` is the owned native region, live for the `&self` borrow; the query only reads the
        // stored point count.
        let num_points = unsafe { lh::heif_region_get_polyline_num_points(self.0.as_ptr()) };
        let size = num_points.max(0) as usize;
        let mut points: Vec<(f64, f64)> = Vec::with_capacity(size);
        if size > 0 {
            // SAFETY: The vector reserves the point count just queried from the same immutable region, the
            // context the transform reads is kept alive by `self.0`, and `set_len` covers only the
            // points that call wrote.
            unsafe {
                lh::heif_region_get_polyline_points_transformed(self.0.as_ptr(), image_id, points.as_mut_ptr() as _);
                points.set_len(size);
            }
        }
        points
    }
}

pub struct ReferencedMaskRegion<'a>(OwnedRegion<'a>);

#[allow(unsafe_code)]
impl ReferencedMaskRegion<'_> {
    /// Get a referenced item mask region.
    #[allow(unsafe_code)]
    pub fn referenced_mask(&self) -> (Rectangle, ItemId) {
        let mut rectangle = Rectangle::default();
        let mut item_id = 0;
        // SAFETY: `self.0` is the owned native region, live for the `&self` borrow; libheif only writes the
        // four out rectangle fields and the out item id.
        unsafe {
            lh::heif_region_get_referenced_mask_ID(
                self.0.as_ptr(),
                &mut rectangle.left,
                &mut rectangle.top,
                &mut rectangle.width,
                &mut rectangle.height,
                &mut item_id,
            );
        }
        (rectangle, item_id)
    }
}

pub struct InlineMaskRegion<'a>(OwnedRegion<'a>);

#[allow(unsafe_code)]
impl InlineMaskRegion<'_> {
    /// Get data for an inline mask region.
    ///
    /// This returns the values in the reference coordinate space
    /// (from the parent region item).
    /// The mask location is represented by a left top corner position,
    /// and a size defined by a width and height.
    ///
    /// The mask is held as inline data on the region, one bit per pixel,
    /// the most significant bit first pixel, no padding. If the bit value is
    /// `1`, the corresponding pixel is part of the region. If the bit value
    /// is `0`, the corresponding pixel is not part of the region.
    #[allow(unsafe_code)]
    pub fn inline_mask_data(&self) -> (Rectangle, Vec<u8>) {
        let mut rectangle = Rectangle::default();
        // SAFETY: `self.0` is the owned native region, live for the `&self` borrow; the query only reads the
        // stored mask byte length.
        let size = unsafe { lh::heif_region_get_inline_mask_data_len(self.0.as_ptr()) };
        let mut data: Vec<u8> = Vec::with_capacity(size);
        // SAFETY: The vector reserves the length just queried from the same immutable region, so libheif
        // writes within its capacity; `set_len` runs after that call and only for that length.
        unsafe {
            lh::heif_region_get_inline_mask_data(
                self.0.as_ptr(),
                &mut rectangle.left,
                &mut rectangle.top,
                &mut rectangle.width,
                &mut rectangle.height,
                data.as_mut_ptr(),
            );
            data.set_len(size);
        }
        (rectangle, data)
    }
}

/// A region of a [`RegionItem`].
///
/// Holds a share of the context the region was parsed from, because libheif's transformed
/// getters read that context's current image properties. A context therefore refuses to read
/// another image while any of its regions is alive.
#[non_exhaustive]
pub enum Region<'a> {
    /// Point geometry.
    ///
    /// The region is represented by a single point.
    Point(PointRegion<'a>),
    /// Rectangle geometry.
    ///
    /// The region is represented by a top left position,
    /// and a size defined by a width and height.
    /// All the interior points and the edge are
    /// part of the region.
    Rectangle(RectangleRegion<'a>),
    /// Ellipse geometry.
    ///
    /// The region is represented by a center point,
    /// and radii in the X and Y directions.
    /// All the interior points and the edge are part of the region.
    Ellipse(EllipseRegion<'a>),
    /// Polygon geometry.
    ///
    /// The region is represented by a sequence of points,
    /// which is considered implicitly closed.
    /// All the interior points and the edge are part of the region.
    Polygon(PolygonRegion<'a>),
    /// Polyline geometry.
    ///
    /// The region is represented by a sequence of points, which are not
    /// considered to form a closed surface. Only the edge is part of
    /// the region.
    Polyline(PolylineRegion<'a>),
    /// Referenced mask.
    ///
    /// The mask location is represented by a left-top corner position,
    /// and a size defined by a width and height. The value of each sample
    /// in that mask identifies whether the corresponding pixel is
    /// part of the region.
    ///
    /// The region geometry is described by the pixels in another image item,
    /// which has an item reference of type `mask` from the region item to
    /// the image item containing the mask.
    ///
    /// The image item containing the mask is one of:
    /// - a mask item (see ISO/IEC 23008-12:2022 Section 6.10.2), or a derived
    ///   image from a mask item;
    /// - an image item in monochrome format (4:0:0 chroma);
    /// - an image item in format of color with luma and chroma planes
    ///   (e.g. 4:2:0).
    ///
    /// If the pixel value is equal to the minimum sample
    /// value (e.g. 0 for unsigned integer), the pixel is not part of
    /// the region. If the pixel value is equal to the maximum sample
    /// value (e.g. 255 for 8 bit unsigned integer), the pixel is part of
    /// the region. If the pixel value is between the minimum sample value
    /// and maximum sample value, the pixel value represents
    /// an (application defined) probability that the pixel is part of
    /// the region, where higher pixel values correspond to higher
    /// probability values.
    ReferencedMask(ReferencedMaskRegion<'a>),
    /// Inline mask.
    ///
    /// The region geometry is described by a sequence of bits stored in inline
    /// in the region, one bit per pixel. If the bit value is `1`, the pixel is
    /// part of the region. If the bit value is `0`, the pixel is not part of
    /// the region.
    InlineMask(InlineMaskRegion<'a>),
}

#[allow(unsafe_code)]
impl<'a> Region<'a> {
    #[allow(unsafe_code)]
    fn new(region_ptr: ptr::NonNull<lh::heif_region>, context: &Rc<ContextOwner<'a>>) -> Option<Self> {
        // SAFETY: `region_ptr` is non-null by its type and points at the live region the caller is about to
        // move into `OwnedRegion`; the shared reference lives only for this call.
        let region_type = unsafe { lh::heif_region_get_type(region_ptr.as_ref()) };
        let region = OwnedRegion::new(region_ptr, context);
        match region_type {
            lh::heif_region_type_heif_region_type_point => Some(Region::Point(PointRegion(region))),
            lh::heif_region_type_heif_region_type_rectangle => Some(Region::Rectangle(RectangleRegion(region))),
            lh::heif_region_type_heif_region_type_ellipse => Some(Region::Ellipse(EllipseRegion(region))),
            lh::heif_region_type_heif_region_type_polygon => Some(Region::Polygon(PolygonRegion(region))),
            lh::heif_region_type_heif_region_type_polyline => Some(Region::Polyline(PolylineRegion(region))),
            lh::heif_region_type_heif_region_type_referenced_mask => {
                Some(Region::ReferencedMask(ReferencedMaskRegion(region)))
            }
            lh::heif_region_type_heif_region_type_inline_mask => Some(Region::InlineMask(InlineMaskRegion(region))),
            _ => None,
        }
    }

    #[allow(unsafe_code)]
    fn inner(&self) -> *mut lh::heif_region {
        match self {
            Region::Point(region) => region.0.as_ptr(),
            Region::Rectangle(region) => region.0.as_ptr(),
            Region::Ellipse(region) => region.0.as_ptr(),
            Region::Polygon(region) => region.0.as_ptr(),
            Region::Polyline(region) => region.0.as_ptr(),
            Region::ReferencedMask(region) => region.0.as_ptr(),
            Region::InlineMask(region) => region.0.as_ptr(),
        }
    }
}

#[allow(unsafe_code)]
impl Drop for Region<'_> {
    #[allow(unsafe_code)]
    fn drop(&mut self) {
        // SAFETY: `inner` is the region this value acquired and releases exactly once, before
        // the context share inside the variant is dropped, so the native context is still live.
        unsafe { lh::heif_region_release(self.inner()) }
    }
}

/// Region item.
///
/// See ISO/IEC 23008-12:2022 Section 6.10 "Region items and region annotations"
///
/// Shares ownership of the context's native state and input backing like [`crate::ImageHandle`].
/// The [`Region`] values it returns carry a share of that same owner, because libheif resolves
/// their transformed getters against the context's image properties.
pub struct RegionItem<'a> {
    pub(crate) inner: *mut lh::heif_region_item,
    _context: Rc<ContextOwner<'a>>,
}

#[allow(unsafe_code)]
impl Drop for RegionItem<'_> {
    #[allow(unsafe_code)]
    fn drop(&mut self) {
        // SAFETY: `inner` is the region item this value owns and releases exactly once, before
        // the context share in `_context` is dropped, so the native context is still live.
        unsafe {
            lh::heif_region_item_release(self.inner);
        }
    }
}

#[allow(unsafe_code)]
impl<'a> RegionItem<'a> {
    pub(crate) fn new(inner: ptr::NonNull<lh::heif_region_item>, context: &Rc<ContextOwner<'a>>) -> Self {
        Self {
            inner: inner.as_ptr(),
            _context: Rc::clone(context),
        }
    }

    /// Take ownership of a native region created from this item, sharing this item's context.
    fn own(&self, region: ptr::NonNull<lh::heif_region>) -> OwnedRegion<'a> {
        OwnedRegion::new(region, &self._context)
    }

    #[allow(unsafe_code)]
    pub fn id(&self) -> ItemId {
        // SAFETY: `inner` is a live region item owned by this wrapper and released once in Drop; the query
        // only reads its id.
        unsafe { lh::heif_region_item_get_id(self.inner) }
    }

    /// Get the reference size for a region item.
    ///
    /// The reference size specifies the coordinate space used for
    /// the region items.
    /// When the reference size does not match the image size, the regions
    /// need to be scaled to correspond.
    #[allow(unsafe_code)]
    pub fn reference_size(&self) -> (u32, u32) {
        let mut width = 0;
        let mut height = 0;
        // SAFETY: `inner` is live and both out slots are initialized integers libheif writes through.
        unsafe {
            lh::heif_region_item_get_reference_size(self.inner, &mut width, &mut height);
        }
        (width, height)
    }

    /// Get the regions that are part of a region item.
    #[allow(unsafe_code)]
    pub fn regions(&self) -> Vec<Region<'a>> {
        // SAFETY: `inner` is live and owned by this wrapper; the query only reads the stored region count.
        let num_regions = unsafe { lh::heif_region_item_get_number_of_regions(self.inner) };
        let size = num_regions.max(0) as usize;
        let mut region_ptrs: Vec<*mut lh::heif_region> = Vec::with_capacity(size);
        let mut regions: Vec<Region<'a>> = Vec::with_capacity(size);
        if size > 0 {
            // SAFETY: The vector reserves exactly `num_regions` pointer slots, the bound libheif is given, and
            // `set_len` runs only for that count. Each returned pointer is a region this value adopts.
            unsafe {
                lh::heif_region_item_get_list_of_regions(self.inner, region_ptrs.as_mut_ptr(), num_regions);
                region_ptrs.set_len(size);
            }
            for region_ptr in region_ptrs.into_iter().filter_map(ptr::NonNull::new) {
                if let Some(region) = Region::new(region_ptr, &self._context) {
                    regions.push(region);
                }
            }
        }
        regions
    }

    /// Add a point region to the region item.
    #[allow(unsafe_code)]
    pub fn add_point(&mut self, x: i32, y: i32) -> Result<Region<'a>> {
        let mut region_ptr: *mut lh::heif_region = ptr::null_mut();
        // SAFETY: `self` is exclusively borrowed and `region_ptr` is a writable out slot; on success
        // libheif stores an owned region that `own` adopts.
        let err = unsafe { lh::heif_region_item_add_region_point(self.inner, x, y, &mut region_ptr) };
        HeifError::from_heif_error(err)?;
        Ok(Region::Point(PointRegion(self.own(get_non_null_ptr(region_ptr)?))))
    }

    /// Add a rectangle region to the region item.
    #[allow(unsafe_code)]
    pub fn add_rectangle(&mut self, rectangle: Rectangle) -> Result<Region<'a>> {
        let mut region_ptr: *mut lh::heif_region = ptr::null_mut();
        // SAFETY: `self` is exclusively borrowed and `region_ptr` is a writable out slot; the rectangle is
        // passed by value and on success libheif stores an owned region that `own` adopts.
        let err = unsafe {
            lh::heif_region_item_add_region_rectangle(
                self.inner,
                rectangle.left,
                rectangle.top,
                rectangle.width,
                rectangle.height,
                &mut region_ptr,
            )
        };
        HeifError::from_heif_error(err)?;
        Ok(Region::Rectangle(RectangleRegion(
            self.own(get_non_null_ptr(region_ptr)?),
        )))
    }

    /// Add an ellipse region to the region item.
    #[allow(unsafe_code)]
    pub fn add_ellipse(&mut self, center_x: i32, center_y: i32, radius_x: u32, radius_y: u32) -> Result<Region<'a>> {
        let mut region_ptr: *mut lh::heif_region = ptr::null_mut();
        // SAFETY: `self` is exclusively borrowed and `region_ptr` is a writable out slot; the geometry is
        // passed by value and on success libheif stores an owned region that `own` adopts.
        let err = unsafe {
            lh::heif_region_item_add_region_ellipse(self.inner, center_x, center_y, radius_x, radius_y, &mut region_ptr)
        };
        HeifError::from_heif_error(err)?;
        Ok(Region::Ellipse(EllipseRegion(self.own(get_non_null_ptr(region_ptr)?))))
    }

    /// Add a polygon region to the region item.
    ///
    /// A polygon is a sequence of points that form a closed shape.
    /// The first point does not need to be repeated as the last point.
    /// The points are provided as pairs of (X, Y) coordinates.
    #[allow(unsafe_code)]
    pub fn add_polygon(&mut self, points: &[(i32, i32)]) -> Result<Region<'a>> {
        let mut region_ptr: *mut lh::heif_region = ptr::null_mut();
        // SAFETY: `points` is a live slice whose `(i32, i32)` pairs have the array layout libheif reads for
        // `len` entries, `self` is exclusively borrowed, and `region_ptr` is a writable out slot.
        let err = unsafe {
            lh::heif_region_item_add_region_polygon(
                self.inner,
                points.as_ptr() as _,
                points.len() as _,
                &mut region_ptr,
            )
        };
        HeifError::from_heif_error(err)?;
        Ok(Region::Polygon(PolygonRegion(self.own(get_non_null_ptr(region_ptr)?))))
    }

    /// Add a polyline region to the region item.
    ///
    /// A polyline is a sequence of points that does not form a closed shape.
    /// Even if the polyline is closed, the only points that are part of
    /// the region are those that intersect (even minimally) a one-pixel
    /// line drawn along the polyline.
    /// The points are provided as pairs of (X, Y) coordinates.
    #[allow(unsafe_code)]
    pub fn add_polyline(&mut self, points: &[(i32, i32)]) -> Result<Region<'a>> {
        let mut region_ptr: *mut lh::heif_region = ptr::null_mut();
        // SAFETY: `points` is a live slice whose `(i32, i32)` pairs have the array layout libheif reads for
        // `len` entries, `self` is exclusively borrowed, and `region_ptr` is a writable out slot.
        let err = unsafe {
            lh::heif_region_item_add_region_polyline(
                self.inner,
                points.as_ptr() as _,
                points.len() as _,
                &mut region_ptr,
            )
        };
        HeifError::from_heif_error(err)?;
        Ok(Region::Polyline(PolylineRegion(
            self.own(get_non_null_ptr(region_ptr)?),
        )))
    }

    /// Add a referenced mask region to the region item.
    #[allow(unsafe_code)]
    pub fn add_referenced_mask(&mut self, rectangle: Rectangle, mask_item_id: ItemId) -> Result<Region<'a>> {
        let mut region_ptr: *mut lh::heif_region = ptr::null_mut();
        // SAFETY: `self` is exclusively borrowed and `region_ptr` is a writable out slot; the mask id and
        // rectangle are passed by value.
        let err = unsafe {
            lh::heif_region_item_add_region_referenced_mask(
                self.inner,
                rectangle.left,
                rectangle.top,
                rectangle.width,
                rectangle.height,
                mask_item_id,
                &mut region_ptr,
            )
        };
        HeifError::from_heif_error(err)?;
        Ok(Region::ReferencedMask(ReferencedMaskRegion(
            self.own(get_non_null_ptr(region_ptr)?),
        )))
    }

    /// Add an inline mask region to the region item.
    ///
    /// The region geometry is described by a left top corner position,
    /// and a size defined by width and height.
    ///
    /// The mask is held as inline data on the region, one bit per pixel,
    /// the most significant bit first pixel, no padding. If the bit value
    /// is `1`, the corresponding pixel is part of the region. If the bit
    /// value is `0`, the corresponding pixel is not part of the region.
    #[allow(unsafe_code)]
    pub fn add_inline_mask_data(&mut self, rectangle: Rectangle, mask_data: &[u8]) -> Result<Region<'a>> {
        let mut region_ptr: *mut lh::heif_region = ptr::null_mut();
        // SAFETY: `mask_data` is a live slice of `len` bytes, `self` is exclusively borrowed, and
        // `region_ptr` is a writable out slot.
        let err = unsafe {
            lh::heif_region_item_add_region_inline_mask_data(
                self.inner,
                rectangle.left,
                rectangle.top,
                rectangle.width,
                rectangle.height,
                mask_data.as_ptr(),
                mask_data.len() as _,
                &mut region_ptr,
            )
        };
        HeifError::from_heif_error(err)?;
        Ok(Region::InlineMask(InlineMaskRegion(
            self.own(get_non_null_ptr(region_ptr)?),
        )))
    }

    /// Add an inline mask region image to the region item.
    ///
    /// The region geometry is described by a left top corner position,
    /// and a size defined by width and height.
    ///
    /// The mask data is held as inline data in the region, one bit per pixel.
    /// The provided image is converted to inline data, where any pixel with
    /// a value >= 0x80 becomes part of the mask region. If the image width
    /// is less than the specified `rectangle.width`, it is expanded
    /// to match the `rectangle.width` of the region (zero fill on the right).
    /// If the image height is less than the specified `rectangle.height`,
    /// it is expanded to match the `rectangle.height` of the region
    /// (zero fill on the bottom). If the image width or height is greater
    /// than the `rectangle.width` or `rectangle.height` (correspondingly)
    /// of the region, the image is cropped.
    #[allow(unsafe_code)]
    pub fn add_inline_mask(&mut self, rectangle: Rectangle, image: &Image) -> Result<Region<'a>> {
        let mut region_ptr: *mut lh::heif_region = ptr::null_mut();
        // SAFETY: `image.inner` is the live image the borrow of `image` keeps alive for the call, `self` is
        // exclusively borrowed, and `region_ptr` is a writable out slot.
        let err = unsafe {
            lh::heif_region_item_add_region_inline_mask(
                self.inner,
                rectangle.left,
                rectangle.top,
                rectangle.width,
                rectangle.height,
                image.inner,
                &mut region_ptr,
            )
        };
        HeifError::from_heif_error(err)?;
        Ok(Region::InlineMask(InlineMaskRegion(
            self.own(get_non_null_ptr(region_ptr)?),
        )))
    }
}
