use crate::bindgen::{
    FPDF_ACTION, FPDF_ANNOT_APPEARANCEMODE, FPDF_ANNOTATION, FPDF_ANNOTATION_SUBTYPE, FPDF_ATTACHMENT, FPDF_AVAIL,
    FPDF_BITMAP, FPDF_BOOKMARK, FPDF_BOOL, FPDF_BYTESTRING, FPDF_CLIPPATH, FPDF_COLORSCHEME, FPDF_CharsetFontMap,
    FPDF_DEST, FPDF_DOCUMENT, FPDF_DUPLEXTYPE, FPDF_DWORD, FPDF_FILEACCESS, FPDF_FILEIDTYPE, FPDF_FILEWRITE, FPDF_FONT,
    FPDF_FORMFILLINFO, FPDF_FORMHANDLE, FPDF_GLYPHPATH, FPDF_IMAGEOBJ_METADATA, FPDF_JAVASCRIPT_ACTION,
    FPDF_LIBRARY_CONFIG, FPDF_LINK, FPDF_OBJECT_TYPE, FPDF_PAGE, FPDF_PAGELINK, FPDF_PAGEOBJECT, FPDF_PAGEOBJECTMARK,
    FPDF_PAGERANGE, FPDF_PATHSEGMENT, FPDF_SCHHANDLE, FPDF_SIGNATURE, FPDF_STRING, FPDF_STRUCTELEMENT,
    FPDF_STRUCTELEMENT_ATTR, FPDF_STRUCTTREE, FPDF_SYSFONTINFO, FPDF_TEXT_RENDERMODE, FPDF_TEXTPAGE, FPDF_WCHAR,
    FPDF_WIDESTRING, FPDF_XOBJECT, FPDFANNOT_COLORTYPE, FS_FLOAT, FS_MATRIX, FS_POINTF, FS_QUADPOINTSF, FS_RECTF,
    FS_SIZEF, FX_DOWNLOADHINTS, FX_FILEAVAIL, IFSDK_PAUSE, size_t,
};

use crate::bindgen::FPDF_STRUCTELEMENT_ATTR_VALUE;

use crate::bindings::PdfiumLibraryBindings;
use crate::error::PdfiumError;
use libloading::{Library, Symbol};
use std::ffi::CString;
use std::os::raw::{c_char, c_double, c_float, c_int, c_long, c_uchar, c_uint, c_ulong, c_ushort, c_void};

#[inline]
fn safe_cstring(s: impl Into<Vec<u8>>) -> CString {
    let bytes: Vec<u8> = s.into();
    match CString::new(bytes) {
        Ok(c) => c,
        Err(e) => {
            let mut bytes = e.into_vec();
            bytes.retain(|&b| b != 0);
            CString::new(bytes).expect("bytes with null removed cannot contain null")
        }
    }
}

#[allow(non_snake_case)]
pub(crate) struct DynamicPdfiumBindings {
    #[allow(dead_code)]
    // We take ownership of the libloading::Library to ensure it has the same lifetime
    // as the dynamic bindings we expose, but we never expect to use the library directly
    // inside this crate.
    library: Library,

    // Instead of using the library directly, we cache function pointers to all exposed
    // Pdfium functionality.
    extern_FPDF_InitLibraryWithConfig: unsafe extern "C" fn(config: *const FPDF_LIBRARY_CONFIG),
    extern_FPDF_InitLibrary: unsafe extern "C" fn(),
    extern_FPDF_SetSandBoxPolicy: unsafe extern "C" fn(policy: FPDF_DWORD, enable: FPDF_BOOL),
    extern_FPDF_DestroyLibrary: unsafe extern "C" fn(),
    extern_FPDF_GetLastError: unsafe extern "C" fn() -> c_ulong,
    extern_FPDF_CreateNewDocument: unsafe extern "C" fn() -> FPDF_DOCUMENT,
    extern_FPDF_LoadDocument: unsafe extern "C" fn(file_path: FPDF_STRING, password: FPDF_BYTESTRING) -> FPDF_DOCUMENT,
    extern_FPDF_LoadMemDocument64:
        unsafe extern "C" fn(data_buf: *const c_void, size: c_ulong, password: FPDF_BYTESTRING) -> FPDF_DOCUMENT,
    extern_FPDF_LoadCustomDocument:
        unsafe extern "C" fn(pFileAccess: *mut FPDF_FILEACCESS, password: FPDF_BYTESTRING) -> FPDF_DOCUMENT,
    extern_FPDF_SaveAsCopy:
        unsafe extern "C" fn(document: FPDF_DOCUMENT, pFileWrite: *mut FPDF_FILEWRITE, flags: FPDF_DWORD) -> FPDF_BOOL,
    extern_FPDF_SaveWithVersion: unsafe extern "C" fn(
        document: FPDF_DOCUMENT,
        pFileWrite: *mut FPDF_FILEWRITE,
        flags: FPDF_DWORD,
        fileVersion: c_int,
    ) -> FPDF_BOOL,
    extern_FPDFAvail_Create:
        unsafe extern "C" fn(file_avail: *mut FX_FILEAVAIL, file: *mut FPDF_FILEACCESS) -> FPDF_AVAIL,
    extern_FPDFAvail_Destroy: unsafe extern "C" fn(avail: FPDF_AVAIL),
    extern_FPDFAvail_IsDocAvail: unsafe extern "C" fn(avail: FPDF_AVAIL, hints: *mut FX_DOWNLOADHINTS) -> c_int,
    extern_FPDFAvail_GetDocument: unsafe extern "C" fn(avail: FPDF_AVAIL, password: FPDF_BYTESTRING) -> FPDF_DOCUMENT,
    extern_FPDFAvail_GetFirstPageNum: unsafe extern "C" fn(doc: FPDF_DOCUMENT) -> c_int,
    extern_FPDFAvail_IsPageAvail:
        unsafe extern "C" fn(avail: FPDF_AVAIL, page_index: c_int, hints: *mut FX_DOWNLOADHINTS) -> c_int,
    extern_FPDFAvail_IsFormAvail: unsafe extern "C" fn(avail: FPDF_AVAIL, hints: *mut FX_DOWNLOADHINTS) -> c_int,
    extern_FPDFAvail_IsLinearized: unsafe extern "C" fn(avail: FPDF_AVAIL) -> c_int,
    extern_FPDF_CloseDocument: unsafe extern "C" fn(document: FPDF_DOCUMENT),
    extern_FPDF_DeviceToPage: unsafe extern "C" fn(
        page: FPDF_PAGE,
        start_x: c_int,
        start_y: c_int,
        size_x: c_int,
        size_y: c_int,
        rotate: c_int,
        device_x: c_int,
        device_y: c_int,
        page_x: *mut c_double,
        page_y: *mut c_double,
    ) -> FPDF_BOOL,
    extern_FPDF_PageToDevice: unsafe extern "C" fn(
        page: FPDF_PAGE,
        start_x: c_int,
        start_y: c_int,
        size_x: c_int,
        size_y: c_int,
        rotate: c_int,
        page_x: c_double,
        page_y: c_double,
        device_x: *mut c_int,
        device_y: *mut c_int,
    ) -> FPDF_BOOL,
    extern_FPDF_GetFileVersion: unsafe extern "C" fn(doc: FPDF_DOCUMENT, fileVersion: *mut c_int) -> FPDF_BOOL,
    extern_FPDF_GetFileIdentifier: unsafe extern "C" fn(
        document: FPDF_DOCUMENT,
        id_type: FPDF_FILEIDTYPE,
        buffer: *mut c_void,
        buflen: c_ulong,
    ) -> c_ulong,
    extern_FPDF_GetMetaText: unsafe extern "C" fn(
        document: FPDF_DOCUMENT,
        tag: FPDF_BYTESTRING,
        buffer: *mut c_void,
        buflen: c_ulong,
    ) -> c_ulong,
    extern_FPDF_DocumentHasValidCrossReferenceTable: unsafe extern "C" fn(document: FPDF_DOCUMENT) -> FPDF_BOOL,
    extern_FPDF_GetTrailerEnds:
        unsafe extern "C" fn(document: FPDF_DOCUMENT, buffer: *mut c_uint, length: c_ulong) -> c_ulong,
    extern_FPDF_GetDocPermissions: unsafe extern "C" fn(document: FPDF_DOCUMENT) -> c_ulong,
    extern_FPDF_GetDocUserPermissions: unsafe extern "C" fn(document: FPDF_DOCUMENT) -> c_ulong,
    extern_FPDF_GetSecurityHandlerRevision: unsafe extern "C" fn(document: FPDF_DOCUMENT) -> c_int,
    extern_FPDF_GetPageCount: unsafe extern "C" fn(document: FPDF_DOCUMENT) -> c_int,
    extern_FPDF_LoadPage: unsafe extern "C" fn(document: FPDF_DOCUMENT, page_index: c_int) -> FPDF_PAGE,
    extern_FPDF_ClosePage: unsafe extern "C" fn(page: FPDF_PAGE),
    extern_FPDF_RenderPageBitmapWithColorScheme_Start: unsafe extern "C" fn(
        bitmap: FPDF_BITMAP,
        page: FPDF_PAGE,
        start_x: c_int,
        start_y: c_int,
        size_x: c_int,
        size_y: c_int,
        rotate: c_int,
        flags: c_int,
        color_scheme: *const FPDF_COLORSCHEME,
        pause: *mut IFSDK_PAUSE,
    ) -> c_int,
    extern_FPDF_RenderPageBitmap_Start: unsafe extern "C" fn(
        bitmap: FPDF_BITMAP,
        page: FPDF_PAGE,
        start_x: c_int,
        start_y: c_int,
        size_x: c_int,
        size_y: c_int,
        rotate: c_int,
        flags: c_int,
        pause: *mut IFSDK_PAUSE,
    ) -> c_int,
    extern_FPDF_RenderPage_Continue: unsafe extern "C" fn(page: FPDF_PAGE, pause: *mut IFSDK_PAUSE) -> c_int,
    extern_FPDF_RenderPage_Close: unsafe extern "C" fn(page: FPDF_PAGE),
    extern_FPDF_ImportPagesByIndex: unsafe extern "C" fn(
        dest_doc: FPDF_DOCUMENT,
        src_doc: FPDF_DOCUMENT,
        page_indices: *const c_int,
        length: c_ulong,
        index: c_int,
    ) -> FPDF_BOOL,
    extern_FPDF_ImportPages: unsafe extern "C" fn(
        dest_doc: FPDF_DOCUMENT,
        src_doc: FPDF_DOCUMENT,
        pagerange: FPDF_BYTESTRING,
        index: c_int,
    ) -> FPDF_BOOL,
    extern_FPDF_ImportNPagesToOne: unsafe extern "C" fn(
        src_doc: FPDF_DOCUMENT,
        output_width: c_float,
        output_height: c_float,
        num_pages_on_x_axis: size_t,
        num_pages_on_y_axis: size_t,
    ) -> FPDF_DOCUMENT,
    extern_FPDF_GetPageLabel: unsafe extern "C" fn(
        document: FPDF_DOCUMENT,
        page_index: c_int,
        buffer: *mut c_void,
        buflen: c_ulong,
    ) -> c_ulong,
    extern_FPDF_GetPageBoundingBox: unsafe extern "C" fn(page: FPDF_PAGE, rect: *mut FS_RECTF) -> FPDF_BOOL,
    extern_FPDF_GetPageSizeByIndexF:
        unsafe extern "C" fn(page: FPDF_DOCUMENT, page_index: c_int, size: *mut FS_SIZEF) -> FPDF_BOOL,
    extern_FPDF_GetPageSizeByIndex:
        unsafe extern "C" fn(document: FPDF_DOCUMENT, page_index: c_int, width: *mut f64, height: *mut f64) -> c_int,
    extern_FPDF_NewXObjectFromPage:
        unsafe extern "C" fn(dest_doc: FPDF_DOCUMENT, src_doc: FPDF_DOCUMENT, src_page_index: c_int) -> FPDF_XOBJECT,
    extern_FPDF_CloseXObject: unsafe extern "C" fn(xobject: FPDF_XOBJECT),
    extern_FPDF_NewFormObjectFromXObject: unsafe extern "C" fn(xobject: FPDF_XOBJECT) -> FPDF_PAGEOBJECT,
    extern_FPDF_CopyViewerPreferences:
        unsafe extern "C" fn(dest_doc: FPDF_DOCUMENT, src_doc: FPDF_DOCUMENT) -> FPDF_BOOL,
    extern_FPDF_GetPageWidth: unsafe extern "C" fn(page: FPDF_PAGE) -> f64,
    extern_FPDF_GetPageHeight: unsafe extern "C" fn(page: FPDF_PAGE) -> f64,
    extern_FPDF_GetPageWidthF: unsafe extern "C" fn(page: FPDF_PAGE) -> c_float,
    extern_FPDF_GetPageHeightF: unsafe extern "C" fn(page: FPDF_PAGE) -> c_float,
    extern_FPDFText_GetCharIndexFromTextIndex:
        unsafe extern "C" fn(text_page: FPDF_TEXTPAGE, nTextIndex: c_int) -> c_int,
    extern_FPDFText_GetTextIndexFromCharIndex:
        unsafe extern "C" fn(text_page: FPDF_TEXTPAGE, nCharIndex: c_int) -> c_int,
    extern_FPDF_GetSignatureCount: unsafe extern "C" fn(document: FPDF_DOCUMENT) -> c_int,
    extern_FPDF_GetSignatureObject: unsafe extern "C" fn(document: FPDF_DOCUMENT, index: c_int) -> FPDF_SIGNATURE,
    extern_FPDFSignatureObj_GetContents:
        unsafe extern "C" fn(signature: FPDF_SIGNATURE, buffer: *mut c_void, length: c_ulong) -> c_ulong,
    extern_FPDFSignatureObj_GetByteRange:
        unsafe extern "C" fn(signature: FPDF_SIGNATURE, buffer: *mut c_int, length: c_ulong) -> c_ulong,
    extern_FPDFSignatureObj_GetSubFilter:
        unsafe extern "C" fn(signature: FPDF_SIGNATURE, buffer: *mut c_char, length: c_ulong) -> c_ulong,
    extern_FPDFSignatureObj_GetReason:
        unsafe extern "C" fn(signature: FPDF_SIGNATURE, buffer: *mut c_void, length: c_ulong) -> c_ulong,
    extern_FPDFSignatureObj_GetTime:
        unsafe extern "C" fn(signature: FPDF_SIGNATURE, buffer: *mut c_char, length: c_ulong) -> c_ulong,
    extern_FPDFSignatureObj_GetDocMDPPermission: unsafe extern "C" fn(signature: FPDF_SIGNATURE) -> c_uint,
    extern_FPDF_StructTree_GetForPage: unsafe extern "C" fn(page: FPDF_PAGE) -> FPDF_STRUCTTREE,
    extern_FPDF_StructTree_Close: unsafe extern "C" fn(struct_tree: FPDF_STRUCTTREE),
    extern_FPDF_StructTree_CountChildren: unsafe extern "C" fn(struct_tree: FPDF_STRUCTTREE) -> c_int,
    extern_FPDF_StructTree_GetChildAtIndex:
        unsafe extern "C" fn(struct_tree: FPDF_STRUCTTREE, index: c_int) -> FPDF_STRUCTELEMENT,
    extern_FPDF_StructElement_GetAltText:
        unsafe extern "C" fn(struct_element: FPDF_STRUCTELEMENT, buffer: *mut c_void, buflen: c_ulong) -> c_ulong,
    extern_FPDF_StructElement_GetActualText:
        unsafe extern "C" fn(struct_element: FPDF_STRUCTELEMENT, buffer: *mut c_void, buflen: c_ulong) -> c_ulong,
    extern_FPDF_StructElement_GetID:
        unsafe extern "C" fn(struct_element: FPDF_STRUCTELEMENT, buffer: *mut c_void, buflen: c_ulong) -> c_ulong,
    extern_FPDF_StructElement_GetLang:
        unsafe extern "C" fn(struct_element: FPDF_STRUCTELEMENT, buffer: *mut c_void, buflen: c_ulong) -> c_ulong,
    extern_FPDF_StructElement_GetStringAttribute: unsafe extern "C" fn(
        struct_element: FPDF_STRUCTELEMENT,
        attr_name: FPDF_BYTESTRING,
        buffer: *mut c_void,
        buflen: c_ulong,
    ) -> c_ulong,
    extern_FPDF_StructElement_GetMarkedContentID: unsafe extern "C" fn(struct_element: FPDF_STRUCTELEMENT) -> c_int,
    extern_FPDF_StructElement_GetType:
        unsafe extern "C" fn(struct_element: FPDF_STRUCTELEMENT, buffer: *mut c_void, buflen: c_ulong) -> c_ulong,
    extern_FPDF_StructElement_GetObjType:
        unsafe extern "C" fn(struct_element: FPDF_STRUCTELEMENT, buffer: *mut c_void, buflen: c_ulong) -> c_ulong,
    extern_FPDF_StructElement_GetTitle:
        unsafe extern "C" fn(struct_element: FPDF_STRUCTELEMENT, buffer: *mut c_void, buflen: c_ulong) -> c_ulong,
    extern_FPDF_StructElement_CountChildren: unsafe extern "C" fn(struct_element: FPDF_STRUCTELEMENT) -> c_int,
    extern_FPDF_StructElement_GetChildAtIndex:
        unsafe extern "C" fn(struct_element: FPDF_STRUCTELEMENT, index: c_int) -> FPDF_STRUCTELEMENT,
    extern_FPDF_StructElement_GetChildMarkedContentID:
        unsafe extern "C" fn(struct_element: FPDF_STRUCTELEMENT, index: c_int) -> c_int,
    extern_FPDF_StructElement_GetParent: unsafe extern "C" fn(struct_element: FPDF_STRUCTELEMENT) -> FPDF_STRUCTELEMENT,
    extern_FPDF_StructElement_GetAttributeCount: unsafe extern "C" fn(struct_element: FPDF_STRUCTELEMENT) -> c_int,
    extern_FPDF_StructElement_GetAttributeAtIndex:
        unsafe extern "C" fn(struct_element: FPDF_STRUCTELEMENT, index: c_int) -> FPDF_STRUCTELEMENT_ATTR,
    extern_FPDF_StructElement_Attr_GetCount: unsafe extern "C" fn(struct_attribute: FPDF_STRUCTELEMENT_ATTR) -> c_int,
    extern_FPDF_StructElement_Attr_GetName: unsafe extern "C" fn(
        struct_attribute: FPDF_STRUCTELEMENT_ATTR,
        index: c_int,
        buffer: *mut c_void,
        buflen: c_ulong,
        out_buflen: *mut c_ulong,
    ) -> FPDF_BOOL,
    extern_FPDF_StructElement_Attr_GetValue: unsafe extern "C" fn(
        struct_attribute: FPDF_STRUCTELEMENT_ATTR,
        name: FPDF_BYTESTRING,
    ) -> FPDF_STRUCTELEMENT_ATTR_VALUE,
    extern_FPDF_StructElement_Attr_GetType:
        unsafe extern "C" fn(value: FPDF_STRUCTELEMENT_ATTR_VALUE) -> FPDF_OBJECT_TYPE,
    extern_FPDF_StructElement_Attr_GetBooleanValue:
        unsafe extern "C" fn(value: FPDF_STRUCTELEMENT_ATTR_VALUE, out_value: *mut FPDF_BOOL) -> FPDF_BOOL,
    extern_FPDF_StructElement_Attr_GetNumberValue:
        unsafe extern "C" fn(value: FPDF_STRUCTELEMENT_ATTR_VALUE, out_value: *mut f32) -> FPDF_BOOL,
    extern_FPDF_StructElement_Attr_GetStringValue: unsafe extern "C" fn(
        value: FPDF_STRUCTELEMENT_ATTR_VALUE,
        buffer: *mut c_void,
        buflen: c_ulong,
        out_buflen: *mut c_ulong,
    ) -> FPDF_BOOL,
    extern_FPDF_StructElement_Attr_GetBlobValue: unsafe extern "C" fn(
        value: FPDF_STRUCTELEMENT_ATTR_VALUE,
        buffer: *mut c_void,
        buflen: c_ulong,
        out_buflen: *mut c_ulong,
    ) -> FPDF_BOOL,
    extern_FPDF_StructElement_Attr_CountChildren: unsafe extern "C" fn(value: FPDF_STRUCTELEMENT_ATTR_VALUE) -> c_int,
    extern_FPDF_StructElement_Attr_GetChildAtIndex:
        unsafe extern "C" fn(value: FPDF_STRUCTELEMENT_ATTR_VALUE, index: c_int) -> FPDF_STRUCTELEMENT_ATTR_VALUE,
    extern_FPDF_StructElement_GetMarkedContentIdCount:
        unsafe extern "C" fn(struct_element: FPDF_STRUCTELEMENT) -> c_int,
    extern_FPDF_StructElement_GetMarkedContentIdAtIndex:
        unsafe extern "C" fn(struct_element: FPDF_STRUCTELEMENT, index: c_int) -> c_int,
    extern_FPDFPage_New: unsafe extern "C" fn(
        document: FPDF_DOCUMENT,
        page_index: c_int,
        width: c_double,
        height: c_double,
    ) -> FPDF_PAGE,
    extern_FPDFPage_Delete: unsafe extern "C" fn(document: FPDF_DOCUMENT, page_index: c_int),
    extern_FPDF_MovePages: unsafe extern "C" fn(
        document: FPDF_DOCUMENT,
        page_indices: *const c_int,
        page_indices_len: c_ulong,
        dest_page_index: c_int,
    ) -> FPDF_BOOL,
    extern_FPDFPage_GetRotation: unsafe extern "C" fn(page: FPDF_PAGE) -> c_int,
    extern_FPDFPage_SetRotation: unsafe extern "C" fn(page: FPDF_PAGE, rotate: c_int),
    extern_FPDFPage_GetMediaBox: unsafe extern "C" fn(
        page: FPDF_PAGE,
        left: *mut c_float,
        bottom: *mut c_float,
        right: *mut c_float,
        top: *mut c_float,
    ) -> FPDF_BOOL,
    extern_FPDFPage_GetCropBox: unsafe extern "C" fn(
        page: FPDF_PAGE,
        left: *mut c_float,
        bottom: *mut c_float,
        right: *mut c_float,
        top: *mut c_float,
    ) -> FPDF_BOOL,
    extern_FPDFPage_GetBleedBox: unsafe extern "C" fn(
        page: FPDF_PAGE,
        left: *mut c_float,
        bottom: *mut c_float,
        right: *mut c_float,
        top: *mut c_float,
    ) -> FPDF_BOOL,
    extern_FPDFPage_GetTrimBox: unsafe extern "C" fn(
        page: FPDF_PAGE,
        left: *mut c_float,
        bottom: *mut c_float,
        right: *mut c_float,
        top: *mut c_float,
    ) -> FPDF_BOOL,
    extern_FPDFPage_GetArtBox: unsafe extern "C" fn(
        page: FPDF_PAGE,
        left: *mut c_float,
        bottom: *mut c_float,
        right: *mut c_float,
        top: *mut c_float,
    ) -> FPDF_BOOL,
    extern_FPDFPage_SetMediaBox:
        unsafe extern "C" fn(page: FPDF_PAGE, left: c_float, bottom: c_float, right: c_float, top: c_float),
    extern_FPDFPage_SetCropBox:
        unsafe extern "C" fn(page: FPDF_PAGE, left: c_float, bottom: c_float, right: c_float, top: c_float),
    extern_FPDFPage_SetBleedBox:
        unsafe extern "C" fn(page: FPDF_PAGE, left: c_float, bottom: c_float, right: c_float, top: c_float),
    extern_FPDFPage_SetTrimBox:
        unsafe extern "C" fn(page: FPDF_PAGE, left: c_float, bottom: c_float, right: c_float, top: c_float),
    extern_FPDFPage_SetArtBox:
        unsafe extern "C" fn(page: FPDF_PAGE, left: c_float, bottom: c_float, right: c_float, top: c_float),
    extern_FPDFPage_TransFormWithClip:
        unsafe extern "C" fn(page: FPDF_PAGE, matrix: *const FS_MATRIX, clipRect: *const FS_RECTF) -> FPDF_BOOL,
    extern_FPDFPageObj_TransformClipPath:
        unsafe extern "C" fn(page_object: FPDF_PAGEOBJECT, a: f64, b: f64, c: f64, d: f64, e: f64, f: f64),
    extern_FPDFPageObj_GetClipPath: unsafe extern "C" fn(page_object: FPDF_PAGEOBJECT) -> FPDF_CLIPPATH,
    extern_FPDFClipPath_CountPaths: unsafe extern "C" fn(clip_path: FPDF_CLIPPATH) -> c_int,
    extern_FPDFClipPath_CountPathSegments: unsafe extern "C" fn(clip_path: FPDF_CLIPPATH, path_index: c_int) -> c_int,
    extern_FPDFClipPath_GetPathSegment:
        unsafe extern "C" fn(clip_path: FPDF_CLIPPATH, path_index: c_int, segment_index: c_int) -> FPDF_PATHSEGMENT,
    extern_FPDF_CreateClipPath: unsafe extern "C" fn(left: f32, bottom: f32, right: f32, top: f32) -> FPDF_CLIPPATH,
    extern_FPDF_DestroyClipPath: unsafe extern "C" fn(clipPath: FPDF_CLIPPATH),
    extern_FPDFPage_InsertClipPath: unsafe extern "C" fn(page: FPDF_PAGE, clipPath: FPDF_CLIPPATH),
    extern_FPDFPage_HasTransparency: unsafe extern "C" fn(page: FPDF_PAGE) -> FPDF_BOOL,
    extern_FPDFPage_GenerateContent: unsafe extern "C" fn(page: FPDF_PAGE) -> FPDF_BOOL,
    extern_FPDFPage_TransformAnnots: unsafe extern "C" fn(
        page: FPDF_PAGE,
        a: c_double,
        b: c_double,
        c: c_double,
        d: c_double,
        e: c_double,
        f: c_double,
    ),
    extern_FPDFBitmap_Create: unsafe extern "C" fn(width: c_int, height: c_int, alpha: c_int) -> FPDF_BITMAP,
    extern_FPDFBitmap_CreateEx: unsafe extern "C" fn(
        width: c_int,
        height: c_int,
        format: c_int,
        first_scan: *mut c_void,
        stride: c_int,
    ) -> FPDF_BITMAP,
    extern_FPDFBitmap_Destroy: unsafe extern "C" fn(bitmap: FPDF_BITMAP),
    extern_FPDFBitmap_GetFormat: unsafe extern "C" fn(bitmap: FPDF_BITMAP) -> c_int,
    extern_FPDFBitmap_FillRect: unsafe extern "C" fn(
        bitmap: FPDF_BITMAP,
        left: c_int,
        top: c_int,
        width: c_int,
        height: c_int,
        color: FPDF_DWORD,
    ) -> FPDF_BOOL,
    extern_FPDFBitmap_GetBuffer: unsafe extern "C" fn(bitmap: FPDF_BITMAP) -> *mut c_void,
    extern_FPDFBitmap_GetWidth: unsafe extern "C" fn(bitmap: FPDF_BITMAP) -> c_int,
    extern_FPDFBitmap_GetHeight: unsafe extern "C" fn(bitmap: FPDF_BITMAP) -> c_int,
    extern_FPDFBitmap_GetStride: unsafe extern "C" fn(bitmap: FPDF_BITMAP) -> c_int,
    extern_FPDF_RenderPageBitmap: unsafe extern "C" fn(
        bitmap: FPDF_BITMAP,
        page: FPDF_PAGE,
        start_x: c_int,
        start_y: c_int,
        size_x: c_int,
        size_y: c_int,
        rotate: c_int,
        flags: c_int,
    ),
    extern_FPDF_RenderPageBitmapWithMatrix: unsafe extern "C" fn(
        bitmap: FPDF_BITMAP,
        page: FPDF_PAGE,
        matrix: *const FS_MATRIX,
        clipping: *const FS_RECTF,
        flags: c_int,
    ),
    extern_FPDFAnnot_IsSupportedSubtype: unsafe extern "C" fn(subtype: FPDF_ANNOTATION_SUBTYPE) -> FPDF_BOOL,
    extern_FPDFPage_CreateAnnot:
        unsafe extern "C" fn(page: FPDF_PAGE, subtype: FPDF_ANNOTATION_SUBTYPE) -> FPDF_ANNOTATION,
    extern_FPDFPage_GetAnnotCount: unsafe extern "C" fn(page: FPDF_PAGE) -> c_int,
    extern_FPDFPage_GetAnnot: unsafe extern "C" fn(page: FPDF_PAGE, index: c_int) -> FPDF_ANNOTATION,
    extern_FPDFPage_GetAnnotIndex: unsafe extern "C" fn(page: FPDF_PAGE, annot: FPDF_ANNOTATION) -> c_int,
    extern_FPDFPage_CloseAnnot: unsafe extern "C" fn(annot: FPDF_ANNOTATION),
    extern_FPDFPage_RemoveAnnot: unsafe extern "C" fn(page: FPDF_PAGE, index: c_int) -> FPDF_BOOL,
    extern_FPDFAnnot_GetSubtype: unsafe extern "C" fn(annot: FPDF_ANNOTATION) -> FPDF_ANNOTATION_SUBTYPE,
    extern_FPDFAnnot_IsObjectSupportedSubtype: unsafe extern "C" fn(subtype: FPDF_ANNOTATION_SUBTYPE) -> FPDF_BOOL,
    extern_FPDFAnnot_UpdateObject: unsafe extern "C" fn(annot: FPDF_ANNOTATION, obj: FPDF_PAGEOBJECT) -> FPDF_BOOL,
    extern_FPDFAnnot_AddInkStroke:
        unsafe extern "C" fn(annot: FPDF_ANNOTATION, points: *const FS_POINTF, point_count: size_t) -> c_int,
    extern_FPDFAnnot_RemoveInkList: unsafe extern "C" fn(annot: FPDF_ANNOTATION) -> FPDF_BOOL,
    extern_FPDFAnnot_AppendObject: unsafe extern "C" fn(annot: FPDF_ANNOTATION, obj: FPDF_PAGEOBJECT) -> FPDF_BOOL,
    extern_FPDFAnnot_GetObjectCount: unsafe extern "C" fn(annot: FPDF_ANNOTATION) -> c_int,
    extern_FPDFAnnot_GetObject: unsafe extern "C" fn(annot: FPDF_ANNOTATION, index: c_int) -> FPDF_PAGEOBJECT,
    extern_FPDFAnnot_RemoveObject: unsafe extern "C" fn(annot: FPDF_ANNOTATION, index: c_int) -> FPDF_BOOL,
    extern_FPDFAnnot_SetColor: unsafe extern "C" fn(
        annot: FPDF_ANNOTATION,
        color_type: FPDFANNOT_COLORTYPE,
        R: c_uint,
        G: c_uint,
        B: c_uint,
        A: c_uint,
    ) -> FPDF_BOOL,
    extern_FPDFAnnot_GetColor: unsafe extern "C" fn(
        annot: FPDF_ANNOTATION,
        color_type: FPDFANNOT_COLORTYPE,
        R: *mut c_uint,
        G: *mut c_uint,
        B: *mut c_uint,
        A: *mut c_uint,
    ) -> FPDF_BOOL,
    extern_FPDFAnnot_HasAttachmentPoints: unsafe extern "C" fn(annot: FPDF_ANNOTATION) -> FPDF_BOOL,
    extern_FPDFAnnot_SetAttachmentPoints: unsafe extern "C" fn(
        annot: FPDF_ANNOTATION,
        quad_index: size_t,
        quad_points: *const FS_QUADPOINTSF,
    ) -> FPDF_BOOL,
    extern_FPDFAnnot_AppendAttachmentPoints:
        unsafe extern "C" fn(annot: FPDF_ANNOTATION, quad_points: *const FS_QUADPOINTSF) -> FPDF_BOOL,
    extern_FPDFAnnot_CountAttachmentPoints: unsafe extern "C" fn(annot: FPDF_ANNOTATION) -> size_t,
    extern_FPDFAnnot_GetAttachmentPoints:
        unsafe extern "C" fn(annot: FPDF_ANNOTATION, quad_index: size_t, quad_points: *mut FS_QUADPOINTSF) -> FPDF_BOOL,
    extern_FPDFAnnot_SetRect: unsafe extern "C" fn(annot: FPDF_ANNOTATION, rect: *const FS_RECTF) -> FPDF_BOOL,
    extern_FPDFAnnot_GetRect: unsafe extern "C" fn(annot: FPDF_ANNOTATION, rect: *mut FS_RECTF) -> FPDF_BOOL,
    extern_FPDFAnnot_GetVertices:
        unsafe extern "C" fn(annot: FPDF_ANNOTATION, buffer: *mut FS_POINTF, length: c_ulong) -> c_ulong,
    extern_FPDFAnnot_GetInkListCount: unsafe extern "C" fn(annot: FPDF_ANNOTATION) -> c_ulong,
    extern_FPDFAnnot_GetInkListPath: unsafe extern "C" fn(
        annot: FPDF_ANNOTATION,
        path_index: c_ulong,
        buffer: *mut FS_POINTF,
        length: c_ulong,
    ) -> c_ulong,
    extern_FPDFAnnot_GetLine:
        unsafe extern "C" fn(annot: FPDF_ANNOTATION, start: *mut FS_POINTF, end: *mut FS_POINTF) -> FPDF_BOOL,
    extern_FPDFAnnot_SetBorder: unsafe extern "C" fn(
        annot: FPDF_ANNOTATION,
        horizontal_radius: f32,
        vertical_radius: f32,
        border_width: f32,
    ) -> FPDF_BOOL,
    extern_FPDFAnnot_GetBorder: unsafe extern "C" fn(
        annot: FPDF_ANNOTATION,
        horizontal_radius: *mut f32,
        vertical_radius: *mut f32,
        border_width: *mut f32,
    ) -> FPDF_BOOL,
    extern_FPDFAnnot_GetFormAdditionalActionJavaScript: unsafe extern "C" fn(
        hHandle: FPDF_FORMHANDLE,
        annot: FPDF_ANNOTATION,
        event: c_int,
        buffer: *mut FPDF_WCHAR,
        buflen: c_ulong,
    ) -> c_ulong,
    extern_FPDFAnnot_GetFormFieldAlternateName: unsafe extern "C" fn(
        hHandle: FPDF_FORMHANDLE,
        annot: FPDF_ANNOTATION,
        buffer: *mut FPDF_WCHAR,
        buflen: c_ulong,
    ) -> c_ulong,
    extern_FPDFAnnot_HasKey: unsafe extern "C" fn(annot: FPDF_ANNOTATION, key: FPDF_BYTESTRING) -> FPDF_BOOL,
    extern_FPDFAnnot_GetValueType:
        unsafe extern "C" fn(annot: FPDF_ANNOTATION, key: FPDF_BYTESTRING) -> FPDF_OBJECT_TYPE,
    extern_FPDFAnnot_SetStringValue:
        unsafe extern "C" fn(annot: FPDF_ANNOTATION, key: FPDF_BYTESTRING, value: FPDF_WIDESTRING) -> FPDF_BOOL,
    extern_FPDFAnnot_GetStringValue: unsafe extern "C" fn(
        annot: FPDF_ANNOTATION,
        key: FPDF_BYTESTRING,
        buffer: *mut FPDF_WCHAR,
        buflen: c_ulong,
    ) -> c_ulong,
    extern_FPDFAnnot_GetNumberValue:
        unsafe extern "C" fn(annot: FPDF_ANNOTATION, key: FPDF_BYTESTRING, value: *mut f32) -> FPDF_BOOL,
    extern_FPDFAnnot_SetAP: unsafe extern "C" fn(
        annot: FPDF_ANNOTATION,
        appearanceMode: FPDF_ANNOT_APPEARANCEMODE,
        value: FPDF_WIDESTRING,
    ) -> FPDF_BOOL,
    extern_FPDFAnnot_GetAP: unsafe extern "C" fn(
        annot: FPDF_ANNOTATION,
        appearanceMode: FPDF_ANNOT_APPEARANCEMODE,
        buffer: *mut FPDF_WCHAR,
        buflen: c_ulong,
    ) -> c_ulong,
    extern_FPDFAnnot_GetLinkedAnnot:
        unsafe extern "C" fn(annot: FPDF_ANNOTATION, key: FPDF_BYTESTRING) -> FPDF_ANNOTATION,
    extern_FPDFAnnot_GetFlags: unsafe extern "C" fn(annot: FPDF_ANNOTATION) -> c_int,
    extern_FPDFAnnot_SetFlags: unsafe extern "C" fn(annot: FPDF_ANNOTATION, flags: c_int) -> FPDF_BOOL,
    extern_FPDFAnnot_GetFormFieldFlags: unsafe extern "C" fn(handle: FPDF_FORMHANDLE, annot: FPDF_ANNOTATION) -> c_int,
    extern_FPDFAnnot_SetFormFieldFlags:
        unsafe extern "C" fn(handle: FPDF_FORMHANDLE, annot: FPDF_ANNOTATION, flags: c_int) -> FPDF_BOOL,
    extern_FPDFAnnot_GetFormFieldAtPoint:
        unsafe extern "C" fn(hHandle: FPDF_FORMHANDLE, page: FPDF_PAGE, point: *const FS_POINTF) -> FPDF_ANNOTATION,
    extern_FPDFAnnot_GetFormFieldName: unsafe extern "C" fn(
        hHandle: FPDF_FORMHANDLE,
        annot: FPDF_ANNOTATION,
        buffer: *mut FPDF_WCHAR,
        buflen: c_ulong,
    ) -> c_ulong,
    extern_FPDFAnnot_GetFormFieldType: unsafe extern "C" fn(hHandle: FPDF_FORMHANDLE, annot: FPDF_ANNOTATION) -> c_int,
    extern_FPDFAnnot_GetFormFieldValue: unsafe extern "C" fn(
        hHandle: FPDF_FORMHANDLE,
        annot: FPDF_ANNOTATION,
        buffer: *mut FPDF_WCHAR,
        buflen: c_ulong,
    ) -> c_ulong,
    extern_FPDFAnnot_GetOptionCount: unsafe extern "C" fn(hHandle: FPDF_FORMHANDLE, annot: FPDF_ANNOTATION) -> c_int,
    extern_FPDFAnnot_GetOptionLabel: unsafe extern "C" fn(
        hHandle: FPDF_FORMHANDLE,
        annot: FPDF_ANNOTATION,
        index: c_int,
        buffer: *mut FPDF_WCHAR,
        buflen: c_ulong,
    ) -> c_ulong,
    extern_FPDFAnnot_IsOptionSelected:
        unsafe extern "C" fn(handle: FPDF_FORMHANDLE, annot: FPDF_ANNOTATION, index: c_int) -> FPDF_BOOL,
    extern_FPDFAnnot_GetFontSize:
        unsafe extern "C" fn(hHandle: FPDF_FORMHANDLE, annot: FPDF_ANNOTATION, value: *mut f32) -> FPDF_BOOL,
    extern_FPDFAnnot_SetFontColor: unsafe extern "C" fn(
        handle: FPDF_FORMHANDLE,
        annot: FPDF_ANNOTATION,
        R: c_uint,
        G: c_uint,
        B: c_uint,
    ) -> FPDF_BOOL,
    extern_FPDFAnnot_GetFontColor: unsafe extern "C" fn(
        hHandle: FPDF_FORMHANDLE,
        annot: FPDF_ANNOTATION,
        R: *mut c_uint,
        G: *mut c_uint,
        B: *mut c_uint,
    ) -> FPDF_BOOL,
    extern_FPDFAnnot_IsChecked: unsafe extern "C" fn(hHandle: FPDF_FORMHANDLE, annot: FPDF_ANNOTATION) -> FPDF_BOOL,
    extern_FPDFAnnot_SetFocusableSubtypes: unsafe extern "C" fn(
        hHandle: FPDF_FORMHANDLE,
        subtypes: *const FPDF_ANNOTATION_SUBTYPE,
        count: size_t,
    ) -> FPDF_BOOL,
    extern_FPDFAnnot_GetFocusableSubtypesCount: unsafe extern "C" fn(hHandle: FPDF_FORMHANDLE) -> c_int,
    extern_FPDFAnnot_GetFocusableSubtypes: unsafe extern "C" fn(
        hHandle: FPDF_FORMHANDLE,
        subtypes: *mut FPDF_ANNOTATION_SUBTYPE,
        count: size_t,
    ) -> FPDF_BOOL,
    extern_FPDFAnnot_GetLink: unsafe extern "C" fn(annot: FPDF_ANNOTATION) -> FPDF_LINK,
    extern_FPDFAnnot_GetFormControlCount:
        unsafe extern "C" fn(hHandle: FPDF_FORMHANDLE, annot: FPDF_ANNOTATION) -> c_int,
    extern_FPDFAnnot_GetFormControlIndex:
        unsafe extern "C" fn(hHandle: FPDF_FORMHANDLE, annot: FPDF_ANNOTATION) -> c_int,
    extern_FPDFAnnot_GetFormFieldExportValue: unsafe extern "C" fn(
        hHandle: FPDF_FORMHANDLE,
        annot: FPDF_ANNOTATION,
        buffer: *mut FPDF_WCHAR,
        buflen: c_ulong,
    ) -> c_ulong,
    extern_FPDFAnnot_SetURI: unsafe extern "C" fn(annot: FPDF_ANNOTATION, uri: *const c_char) -> FPDF_BOOL,
    extern_FPDFAnnot_GetFileAttachment: unsafe extern "C" fn(annot: FPDF_ANNOTATION) -> FPDF_ATTACHMENT,
    extern_FPDFAnnot_AddFileAttachment:
        unsafe extern "C" fn(annot: FPDF_ANNOTATION, name: FPDF_WIDESTRING) -> FPDF_ATTACHMENT,
    extern_FPDFDOC_InitFormFillEnvironment:
        unsafe extern "C" fn(document: FPDF_DOCUMENT, form_info: *mut FPDF_FORMFILLINFO) -> FPDF_FORMHANDLE,
    extern_FPDFDOC_ExitFormFillEnvironment: unsafe extern "C" fn(handle: FPDF_FORMHANDLE),
    extern_FORM_OnAfterLoadPage: unsafe extern "C" fn(page: FPDF_PAGE, handle: FPDF_FORMHANDLE),
    extern_FORM_OnBeforeClosePage: unsafe extern "C" fn(page: FPDF_PAGE, handle: FPDF_FORMHANDLE),
    extern_FPDFDoc_GetPageMode: unsafe extern "C" fn(document: FPDF_DOCUMENT) -> c_int,
    extern_FPDFPage_Flatten: unsafe extern "C" fn(page: FPDF_PAGE, nFlag: c_int) -> c_int,
    extern_FORM_DoDocumentJSAction: unsafe extern "C" fn(hHandle: FPDF_FORMHANDLE),
    extern_FORM_DoDocumentOpenAction: unsafe extern "C" fn(hHandle: FPDF_FORMHANDLE),
    extern_FORM_DoDocumentAAction: unsafe extern "C" fn(hHandle: FPDF_FORMHANDLE, aaType: c_int),
    extern_FORM_DoPageAAction: unsafe extern "C" fn(page: FPDF_PAGE, hHandle: FPDF_FORMHANDLE, aaType: c_int),
    extern_FORM_OnMouseMove: unsafe extern "C" fn(
        hHandle: FPDF_FORMHANDLE,
        page: FPDF_PAGE,
        modifier: c_int,
        page_x: f64,
        page_y: f64,
    ) -> FPDF_BOOL,
    extern_FORM_OnMouseWheel: unsafe extern "C" fn(
        hHandle: FPDF_FORMHANDLE,
        page: FPDF_PAGE,
        modifier: c_int,
        page_coord: *const FS_POINTF,
        delta_x: c_int,
        delta_y: c_int,
    ) -> FPDF_BOOL,
    extern_FORM_OnFocus: unsafe extern "C" fn(
        hHandle: FPDF_FORMHANDLE,
        page: FPDF_PAGE,
        modifier: c_int,
        page_x: f64,
        page_y: f64,
    ) -> FPDF_BOOL,
    extern_FORM_OnLButtonDown: unsafe extern "C" fn(
        hHandle: FPDF_FORMHANDLE,
        page: FPDF_PAGE,
        modifier: c_int,
        page_x: f64,
        page_y: f64,
    ) -> FPDF_BOOL,
    extern_FORM_OnRButtonDown: unsafe extern "C" fn(
        hHandle: FPDF_FORMHANDLE,
        page: FPDF_PAGE,
        modifier: c_int,
        page_x: f64,
        page_y: f64,
    ) -> FPDF_BOOL,
    extern_FORM_OnLButtonUp: unsafe extern "C" fn(
        hHandle: FPDF_FORMHANDLE,
        page: FPDF_PAGE,
        modifier: c_int,
        page_x: f64,
        page_y: f64,
    ) -> FPDF_BOOL,
    extern_FORM_OnRButtonUp: unsafe extern "C" fn(
        hHandle: FPDF_FORMHANDLE,
        page: FPDF_PAGE,
        modifier: c_int,
        page_x: f64,
        page_y: f64,
    ) -> FPDF_BOOL,
    extern_FORM_OnLButtonDoubleClick: unsafe extern "C" fn(
        hHandle: FPDF_FORMHANDLE,
        page: FPDF_PAGE,
        modifier: c_int,
        page_x: f64,
        page_y: f64,
    ) -> FPDF_BOOL,
    extern_FORM_OnKeyDown:
        unsafe extern "C" fn(hHandle: FPDF_FORMHANDLE, page: FPDF_PAGE, nKeyCode: c_int, modifier: c_int) -> FPDF_BOOL,
    extern_FORM_OnKeyUp:
        unsafe extern "C" fn(hHandle: FPDF_FORMHANDLE, page: FPDF_PAGE, nKeyCode: c_int, modifier: c_int) -> FPDF_BOOL,
    extern_FORM_OnChar:
        unsafe extern "C" fn(hHandle: FPDF_FORMHANDLE, page: FPDF_PAGE, nChar: c_int, modifier: c_int) -> FPDF_BOOL,
    extern_FORM_GetFocusedText: unsafe extern "C" fn(
        hHandle: FPDF_FORMHANDLE,
        page: FPDF_PAGE,
        buffer: *mut c_void,
        buflen: c_ulong,
    ) -> c_ulong,
    extern_FORM_GetSelectedText: unsafe extern "C" fn(
        hHandle: FPDF_FORMHANDLE,
        page: FPDF_PAGE,
        buffer: *mut c_void,
        buflen: c_ulong,
    ) -> c_ulong,
    extern_FORM_ReplaceAndKeepSelection:
        unsafe extern "C" fn(hHandle: FPDF_FORMHANDLE, page: FPDF_PAGE, wsText: FPDF_WIDESTRING),
    extern_FORM_ReplaceSelection:
        unsafe extern "C" fn(hHandle: FPDF_FORMHANDLE, page: FPDF_PAGE, wsText: FPDF_WIDESTRING),
    extern_FORM_SelectAllText: unsafe extern "C" fn(hHandle: FPDF_FORMHANDLE, page: FPDF_PAGE) -> FPDF_BOOL,
    extern_FORM_CanUndo: unsafe extern "C" fn(hHandle: FPDF_FORMHANDLE, page: FPDF_PAGE) -> FPDF_BOOL,
    extern_FORM_CanRedo: unsafe extern "C" fn(hHandle: FPDF_FORMHANDLE, page: FPDF_PAGE) -> FPDF_BOOL,
    extern_FORM_Undo: unsafe extern "C" fn(hHandle: FPDF_FORMHANDLE, page: FPDF_PAGE) -> FPDF_BOOL,
    extern_FORM_Redo: unsafe extern "C" fn(hHandle: FPDF_FORMHANDLE, page: FPDF_PAGE) -> FPDF_BOOL,
    extern_FORM_ForceToKillFocus: unsafe extern "C" fn(hHandle: FPDF_FORMHANDLE) -> FPDF_BOOL,
    extern_FORM_GetFocusedAnnot:
        unsafe extern "C" fn(handle: FPDF_FORMHANDLE, page_index: *mut c_int, annot: *mut FPDF_ANNOTATION) -> FPDF_BOOL,
    extern_FORM_SetFocusedAnnot: unsafe extern "C" fn(handle: FPDF_FORMHANDLE, annot: FPDF_ANNOTATION) -> FPDF_BOOL,
    extern_FPDFPage_HasFormFieldAtPoint:
        unsafe extern "C" fn(hHandle: FPDF_FORMHANDLE, page: FPDF_PAGE, page_x: f64, page_y: f64) -> c_int,
    extern_FPDFPage_FormFieldZOrderAtPoint:
        unsafe extern "C" fn(hHandle: FPDF_FORMHANDLE, page: FPDF_PAGE, page_x: f64, page_y: f64) -> c_int,
    extern_FPDF_SetFormFieldHighlightColor:
        unsafe extern "C" fn(handle: FPDF_FORMHANDLE, field_type: c_int, color: c_ulong),
    extern_FPDF_SetFormFieldHighlightAlpha: unsafe extern "C" fn(handle: FPDF_FORMHANDLE, alpha: c_uchar),
    extern_FPDF_RemoveFormFieldHighlight: unsafe extern "C" fn(hHandle: FPDF_FORMHANDLE),
    extern_FPDF_FFLDraw: unsafe extern "C" fn(
        handle: FPDF_FORMHANDLE,
        bitmap: FPDF_BITMAP,
        page: FPDF_PAGE,
        start_x: c_int,
        start_y: c_int,
        size_x: c_int,
        size_y: c_int,
        rotate: c_int,
        flags: c_int,
    ),
    extern_FPDF_GetFormType: unsafe extern "C" fn(document: FPDF_DOCUMENT) -> c_int,
    extern_FORM_SetIndexSelected:
        unsafe extern "C" fn(hHandle: FPDF_FORMHANDLE, page: FPDF_PAGE, index: c_int, selected: FPDF_BOOL) -> FPDF_BOOL,
    extern_FORM_IsIndexSelected:
        unsafe extern "C" fn(hHandle: FPDF_FORMHANDLE, page: FPDF_PAGE, index: c_int) -> FPDF_BOOL,
    extern_FPDF_LoadXFA: unsafe extern "C" fn(document: FPDF_DOCUMENT) -> FPDF_BOOL,
    extern_FPDFDoc_GetJavaScriptActionCount: unsafe extern "C" fn(document: FPDF_DOCUMENT) -> c_int,
    extern_FPDFDoc_GetJavaScriptAction:
        unsafe extern "C" fn(document: FPDF_DOCUMENT, index: c_int) -> FPDF_JAVASCRIPT_ACTION,
    extern_FPDFDoc_CloseJavaScriptAction: unsafe extern "C" fn(javascript: FPDF_JAVASCRIPT_ACTION),
    extern_FPDFJavaScriptAction_GetName:
        unsafe extern "C" fn(javascript: FPDF_JAVASCRIPT_ACTION, buffer: *mut FPDF_WCHAR, buflen: c_ulong) -> c_ulong,
    extern_FPDFJavaScriptAction_GetScript:
        unsafe extern "C" fn(javascript: FPDF_JAVASCRIPT_ACTION, buffer: *mut FPDF_WCHAR, buflen: c_ulong) -> c_ulong,
    extern_FPDF_GetDefaultTTFMap: unsafe extern "C" fn() -> *const FPDF_CharsetFontMap,
    extern_FPDF_GetDefaultTTFMapCount: unsafe extern "C" fn() -> usize,
    extern_FPDF_GetDefaultTTFMapEntry: unsafe extern "C" fn(index: usize) -> *const FPDF_CharsetFontMap,
    extern_FPDF_AddInstalledFont: unsafe extern "C" fn(mapper: *mut c_void, face: *const c_char, charset: c_int),
    extern_FPDF_SetSystemFontInfo: unsafe extern "C" fn(pFontInfo: *mut FPDF_SYSFONTINFO),
    extern_FPDF_GetDefaultSystemFontInfo: unsafe extern "C" fn() -> *mut FPDF_SYSFONTINFO,
    extern_FPDF_FreeDefaultSystemFontInfo: unsafe extern "C" fn(pFontInfo: *mut FPDF_SYSFONTINFO),
    extern_FPDFBookmark_GetFirstChild:
        unsafe extern "C" fn(document: FPDF_DOCUMENT, bookmark: FPDF_BOOKMARK) -> FPDF_BOOKMARK,
    extern_FPDFBookmark_GetNextSibling:
        unsafe extern "C" fn(document: FPDF_DOCUMENT, bookmark: FPDF_BOOKMARK) -> FPDF_BOOKMARK,
    extern_FPDFBookmark_GetTitle:
        unsafe extern "C" fn(bookmark: FPDF_BOOKMARK, buffer: *mut c_void, buflen: c_ulong) -> c_ulong,
    extern_FPDFBookmark_GetCount: unsafe extern "C" fn(bookmark: FPDF_BOOKMARK) -> c_int,
    extern_FPDFBookmark_Find: unsafe extern "C" fn(document: FPDF_DOCUMENT, title: FPDF_WIDESTRING) -> FPDF_BOOKMARK,
    extern_FPDFBookmark_GetDest: unsafe extern "C" fn(document: FPDF_DOCUMENT, bookmark: FPDF_BOOKMARK) -> FPDF_DEST,
    extern_FPDFBookmark_GetAction: unsafe extern "C" fn(bookmark: FPDF_BOOKMARK) -> FPDF_ACTION,
    extern_FPDFAction_GetType: unsafe extern "C" fn(action: FPDF_ACTION) -> c_ulong,
    extern_FPDFAction_GetDest: unsafe extern "C" fn(document: FPDF_DOCUMENT, action: FPDF_ACTION) -> FPDF_DEST,
    extern_FPDFAction_GetFilePath:
        unsafe extern "C" fn(action: FPDF_ACTION, buffer: *mut c_void, buflen: c_ulong) -> c_ulong,
    extern_FPDFAction_GetURIPath: unsafe extern "C" fn(
        document: FPDF_DOCUMENT,
        action: FPDF_ACTION,
        buffer: *mut c_void,
        buflen: c_ulong,
    ) -> c_ulong,
    extern_FPDFDest_GetDestPageIndex: unsafe extern "C" fn(document: FPDF_DOCUMENT, dest: FPDF_DEST) -> c_int,
    extern_FPDFDest_GetView:
        unsafe extern "C" fn(dest: FPDF_DEST, pNumParams: *mut c_ulong, pParams: *mut FS_FLOAT) -> c_ulong,
    extern_FPDFDest_GetLocationInPage: unsafe extern "C" fn(
        dest: FPDF_DEST,
        hasXVal: *mut FPDF_BOOL,
        hasYVal: *mut FPDF_BOOL,
        hasZoomVal: *mut FPDF_BOOL,
        x: *mut FS_FLOAT,
        y: *mut FS_FLOAT,
        zoom: *mut FS_FLOAT,
    ) -> FPDF_BOOL,
    extern_FPDFLink_GetLinkAtPoint: unsafe extern "C" fn(page: FPDF_PAGE, x: c_double, y: c_double) -> FPDF_LINK,
    extern_FPDFLink_GetLinkZOrderAtPoint: unsafe extern "C" fn(page: FPDF_PAGE, x: c_double, y: c_double) -> c_int,
    extern_FPDFLink_GetDest: unsafe extern "C" fn(document: FPDF_DOCUMENT, link: FPDF_LINK) -> FPDF_DEST,
    extern_FPDFLink_GetAction: unsafe extern "C" fn(link: FPDF_LINK) -> FPDF_ACTION,
    extern_FPDFLink_Enumerate:
        unsafe extern "C" fn(page: FPDF_PAGE, start_pos: *mut c_int, link_annot: *mut FPDF_LINK) -> FPDF_BOOL,
    extern_FPDFLink_GetAnnot: unsafe extern "C" fn(page: FPDF_PAGE, link_annot: FPDF_LINK) -> FPDF_ANNOTATION,
    extern_FPDFLink_GetAnnotRect: unsafe extern "C" fn(link_annot: FPDF_LINK, rect: *mut FS_RECTF) -> FPDF_BOOL,
    extern_FPDFLink_CountQuadPoints: unsafe extern "C" fn(link_annot: FPDF_LINK) -> c_int,
    extern_FPDFLink_GetQuadPoints:
        unsafe extern "C" fn(link_annot: FPDF_LINK, quad_index: c_int, quad_points: *mut FS_QUADPOINTSF) -> FPDF_BOOL,
    extern_FPDF_GetPageAAction: unsafe extern "C" fn(page: FPDF_PAGE, aa_type: c_int) -> FPDF_ACTION,
    extern_FPDFText_LoadPage: unsafe extern "C" fn(page: FPDF_PAGE) -> FPDF_TEXTPAGE,
    extern_FPDFText_ClosePage: unsafe extern "C" fn(text_page: FPDF_TEXTPAGE),
    extern_FPDFText_CountChars: unsafe extern "C" fn(text_page: FPDF_TEXTPAGE) -> c_int,
    extern_FPDFText_GetUnicode: unsafe extern "C" fn(text_page: FPDF_TEXTPAGE, index: c_int) -> c_uint,
    extern_FPDFText_GetTextObject: unsafe extern "C" fn(text_page: FPDF_TEXTPAGE, index: c_int) -> FPDF_PAGEOBJECT,
    extern_FPDFText_IsGenerated: unsafe extern "C" fn(text_page: FPDF_TEXTPAGE, index: c_int) -> c_int,
    extern_FPDFText_IsHyphen: unsafe extern "C" fn(text_page: FPDF_TEXTPAGE, index: c_int) -> c_int,
    extern_FPDFText_HasUnicodeMapError: unsafe extern "C" fn(text_page: FPDF_TEXTPAGE, index: c_int) -> c_int,
    extern_FPDFText_GetFontSize: unsafe extern "C" fn(text_page: FPDF_TEXTPAGE, index: c_int) -> c_double,
    extern_FPDFText_GetFontInfo: unsafe extern "C" fn(
        text_page: FPDF_TEXTPAGE,
        index: c_int,
        buffer: *mut c_void,
        buflen: c_ulong,
        flags: *mut c_int,
    ) -> c_ulong,
    extern_FPDFText_GetFontWeight: unsafe extern "C" fn(text_page: FPDF_TEXTPAGE, index: c_int) -> c_int,
    extern_FPDFText_GetFillColor: unsafe extern "C" fn(
        text_page: FPDF_TEXTPAGE,
        index: c_int,
        R: *mut c_uint,
        G: *mut c_uint,
        B: *mut c_uint,
        A: *mut c_uint,
    ) -> FPDF_BOOL,
    extern_FPDFText_GetStrokeColor: unsafe extern "C" fn(
        text_page: FPDF_TEXTPAGE,
        index: c_int,
        R: *mut c_uint,
        G: *mut c_uint,
        B: *mut c_uint,
        A: *mut c_uint,
    ) -> FPDF_BOOL,
    extern_FPDFText_GetCharAngle: unsafe extern "C" fn(text_page: FPDF_TEXTPAGE, index: c_int) -> c_float,
    extern_FPDFText_GetCharBox: unsafe extern "C" fn(
        text_page: FPDF_TEXTPAGE,
        index: c_int,
        left: *mut c_double,
        right: *mut c_double,
        bottom: *mut c_double,
        top: *mut c_double,
    ) -> FPDF_BOOL,
    extern_FPDFText_GetLooseCharBox:
        unsafe extern "C" fn(text_page: FPDF_TEXTPAGE, index: c_int, rect: *mut FS_RECTF) -> FPDF_BOOL,
    extern_FPDFText_GetMatrix:
        unsafe extern "C" fn(text_page: FPDF_TEXTPAGE, index: c_int, matrix: *mut FS_MATRIX) -> FPDF_BOOL,
    extern_FPDFText_GetCharOrigin:
        unsafe extern "C" fn(text_page: FPDF_TEXTPAGE, index: c_int, x: *mut c_double, y: *mut c_double) -> FPDF_BOOL,
    extern_FPDFText_GetCharIndexAtPos: unsafe extern "C" fn(
        text_page: FPDF_TEXTPAGE,
        x: c_double,
        y: c_double,
        xTolerance: c_double,
        yTolerance: c_double,
    ) -> c_int,
    extern_FPDFText_GetText: unsafe extern "C" fn(
        text_page: FPDF_TEXTPAGE,
        start_index: c_int,
        count: c_int,
        result: *mut c_ushort,
    ) -> c_int,
    extern_FPDFText_CountRects:
        unsafe extern "C" fn(text_page: FPDF_TEXTPAGE, start_index: c_int, count: c_int) -> c_int,
    extern_FPDFText_GetRect: unsafe extern "C" fn(
        text_page: FPDF_TEXTPAGE,
        rect_index: c_int,
        left: *mut c_double,
        top: *mut c_double,
        right: *mut c_double,
        bottom: *mut c_double,
    ) -> FPDF_BOOL,
    extern_FPDFText_GetBoundedText: unsafe extern "C" fn(
        text_page: FPDF_TEXTPAGE,
        left: c_double,
        top: c_double,
        right: c_double,
        bottom: c_double,
        buffer: *mut c_ushort,
        buflen: c_int,
    ) -> c_int,
    extern_FPDFText_FindStart: unsafe extern "C" fn(
        text_page: FPDF_TEXTPAGE,
        findwhat: FPDF_WIDESTRING,
        flags: c_ulong,
        start_index: c_int,
    ) -> FPDF_SCHHANDLE,
    extern_FPDFText_FindNext: unsafe extern "C" fn(handle: FPDF_SCHHANDLE) -> FPDF_BOOL,
    extern_FPDFText_FindPrev: unsafe extern "C" fn(handle: FPDF_SCHHANDLE) -> FPDF_BOOL,
    extern_FPDFText_GetSchResultIndex: unsafe extern "C" fn(handle: FPDF_SCHHANDLE) -> c_int,
    extern_FPDFText_GetSchCount: unsafe extern "C" fn(handle: FPDF_SCHHANDLE) -> c_int,
    extern_FPDFText_FindClose: unsafe extern "C" fn(handle: FPDF_SCHHANDLE),
    extern_FPDFLink_LoadWebLinks: unsafe extern "C" fn(text_page: FPDF_TEXTPAGE) -> FPDF_PAGELINK,
    extern_FPDFLink_CountWebLinks: unsafe extern "C" fn(link_page: FPDF_PAGELINK) -> c_int,
    extern_FPDFLink_GetURL: unsafe extern "C" fn(
        link_page: FPDF_PAGELINK,
        link_index: c_int,
        buffer: *mut c_ushort,
        buflen: c_int,
    ) -> c_int,
    extern_FPDFLink_CountRects: unsafe extern "C" fn(link_page: FPDF_PAGELINK, link_index: c_int) -> c_int,
    extern_FPDFLink_GetRect: unsafe extern "C" fn(
        link_page: FPDF_PAGELINK,
        link_index: c_int,
        rect_index: c_int,
        left: *mut c_double,
        top: *mut c_double,
        right: *mut c_double,
        bottom: *mut c_double,
    ) -> FPDF_BOOL,
    extern_FPDFLink_GetTextRange: unsafe extern "C" fn(
        link_page: FPDF_PAGELINK,
        link_index: c_int,
        start_char_index: *mut c_int,
        char_count: *mut c_int,
    ) -> FPDF_BOOL,
    extern_FPDFLink_CloseWebLinks: unsafe extern "C" fn(link_page: FPDF_PAGELINK),
    extern_FPDFPage_GetDecodedThumbnailData:
        unsafe extern "C" fn(page: FPDF_PAGE, buffer: *mut c_void, buflen: c_ulong) -> c_ulong,
    extern_FPDFPage_GetRawThumbnailData:
        unsafe extern "C" fn(page: FPDF_PAGE, buffer: *mut c_void, buflen: c_ulong) -> c_ulong,
    extern_FPDFPage_GetThumbnailAsBitmap: unsafe extern "C" fn(page: FPDF_PAGE) -> FPDF_BITMAP,
    extern_FPDFFormObj_CountObjects: unsafe extern "C" fn(form_object: FPDF_PAGEOBJECT) -> c_int,
    extern_FPDFFormObj_GetObject: unsafe extern "C" fn(form_object: FPDF_PAGEOBJECT, index: c_ulong) -> FPDF_PAGEOBJECT,
    extern_FPDFFormObj_RemoveObject:
        unsafe extern "C" fn(form_object: FPDF_PAGEOBJECT, page_object: FPDF_PAGEOBJECT) -> FPDF_BOOL,
    extern_FPDFPageObj_CreateTextObj:
        unsafe extern "C" fn(document: FPDF_DOCUMENT, font: FPDF_FONT, font_size: c_float) -> FPDF_PAGEOBJECT,
    extern_FPDFTextObj_GetTextRenderMode: unsafe extern "C" fn(text: FPDF_PAGEOBJECT) -> FPDF_TEXT_RENDERMODE,
    extern_FPDFTextObj_SetTextRenderMode:
        unsafe extern "C" fn(text: FPDF_PAGEOBJECT, render_mode: FPDF_TEXT_RENDERMODE) -> FPDF_BOOL,
    extern_FPDFTextObj_GetText: unsafe extern "C" fn(
        text_object: FPDF_PAGEOBJECT,
        text_page: FPDF_TEXTPAGE,
        buffer: *mut FPDF_WCHAR,
        length: c_ulong,
    ) -> c_ulong,
    extern_FPDFTextObj_GetRenderedBitmap: unsafe extern "C" fn(
        document: FPDF_DOCUMENT,
        page: FPDF_PAGE,
        text_object: FPDF_PAGEOBJECT,
        scale: f32,
    ) -> FPDF_BITMAP,
    extern_FPDFTextObj_GetFont: unsafe extern "C" fn(text: FPDF_PAGEOBJECT) -> FPDF_FONT,
    extern_FPDFTextObj_GetFontSize: unsafe extern "C" fn(text: FPDF_PAGEOBJECT, size: *mut c_float) -> FPDF_BOOL,
    extern_FPDFPageObj_NewTextObj:
        unsafe extern "C" fn(document: FPDF_DOCUMENT, font: FPDF_BYTESTRING, font_size: c_float) -> FPDF_PAGEOBJECT,
    extern_FPDFText_SetText: unsafe extern "C" fn(text_object: FPDF_PAGEOBJECT, text: FPDF_WIDESTRING) -> FPDF_BOOL,
    extern_FPDFText_SetCharcodes:
        unsafe extern "C" fn(text_object: FPDF_PAGEOBJECT, charcodes: *const c_uint, count: size_t) -> FPDF_BOOL,
    extern_FPDFText_LoadFont: unsafe extern "C" fn(
        document: FPDF_DOCUMENT,
        data: *const c_uchar,
        size: c_uint,
        font_type: c_int,
        cid: FPDF_BOOL,
    ) -> FPDF_FONT,
    extern_FPDFText_LoadStandardFont: unsafe extern "C" fn(document: FPDF_DOCUMENT, font: FPDF_BYTESTRING) -> FPDF_FONT,
    extern_FPDFText_LoadCidType2Font: unsafe extern "C" fn(
        document: FPDF_DOCUMENT,
        font_data: *const u8,
        font_data_size: u32,
        to_unicode_cmap: FPDF_BYTESTRING,
        cid_to_gid_map_data: *const u8,
        cid_to_gid_map_data_size: u32,
    ) -> FPDF_FONT,
    extern_FPDFFont_Close: unsafe extern "C" fn(font: FPDF_FONT),
    extern_FPDFPath_MoveTo: unsafe extern "C" fn(path: FPDF_PAGEOBJECT, x: c_float, y: c_float) -> FPDF_BOOL,
    extern_FPDFPath_LineTo: unsafe extern "C" fn(path: FPDF_PAGEOBJECT, x: c_float, y: c_float) -> FPDF_BOOL,
    extern_FPDFPath_BezierTo: unsafe extern "C" fn(
        path: FPDF_PAGEOBJECT,
        x1: c_float,
        y1: c_float,
        x2: c_float,
        y2: c_float,
        x3: c_float,
        y3: c_float,
    ) -> FPDF_BOOL,
    extern_FPDFPath_Close: unsafe extern "C" fn(path: FPDF_PAGEOBJECT) -> FPDF_BOOL,
    extern_FPDFPath_SetDrawMode:
        unsafe extern "C" fn(path: FPDF_PAGEOBJECT, fillmode: c_int, stroke: FPDF_BOOL) -> FPDF_BOOL,
    extern_FPDFPath_GetDrawMode:
        unsafe extern "C" fn(path: FPDF_PAGEOBJECT, fillmode: *mut c_int, stroke: *mut FPDF_BOOL) -> FPDF_BOOL,
    extern_FPDFPage_InsertObject: unsafe extern "C" fn(page: FPDF_PAGE, page_obj: FPDF_PAGEOBJECT),
    extern_FPDFPage_InsertObjectAtIndex:
        unsafe extern "C" fn(page: FPDF_PAGE, page_object: FPDF_PAGEOBJECT, index: usize) -> FPDF_BOOL,
    extern_FPDFPage_RemoveObject: unsafe extern "C" fn(page: FPDF_PAGE, page_obj: FPDF_PAGEOBJECT) -> FPDF_BOOL,
    extern_FPDFPage_CountObjects: unsafe extern "C" fn(page: FPDF_PAGE) -> c_int,
    extern_FPDFPage_GetObject: unsafe extern "C" fn(page: FPDF_PAGE, index: c_int) -> FPDF_PAGEOBJECT,
    extern_FPDFPageObj_Destroy: unsafe extern "C" fn(page_obj: FPDF_PAGEOBJECT),
    extern_FPDFPageObj_HasTransparency: unsafe extern "C" fn(page_object: FPDF_PAGEOBJECT) -> FPDF_BOOL,
    extern_FPDFPageObj_GetType: unsafe extern "C" fn(page_object: FPDF_PAGEOBJECT) -> c_int,
    extern_FPDFPageObj_GetIsActive:
        unsafe extern "C" fn(page_object: FPDF_PAGEOBJECT, active: *mut FPDF_BOOL) -> FPDF_BOOL,
    extern_FPDFPageObj_SetIsActive: unsafe extern "C" fn(page_object: FPDF_PAGEOBJECT, active: FPDF_BOOL) -> FPDF_BOOL,
    extern_FPDFPageObj_Transform: unsafe extern "C" fn(
        page_object: FPDF_PAGEOBJECT,
        a: c_double,
        b: c_double,
        c: c_double,
        d: c_double,
        e: c_double,
        f: c_double,
    ),
    extern_FPDFPageObj_TransformF:
        unsafe extern "C" fn(page_object: FPDF_PAGEOBJECT, matrix: *const FS_MATRIX) -> FPDF_BOOL,
    extern_FPDFPageObj_GetMatrix:
        unsafe extern "C" fn(page_object: FPDF_PAGEOBJECT, matrix: *mut FS_MATRIX) -> FPDF_BOOL,
    extern_FPDFPageObj_SetMatrix: unsafe extern "C" fn(path: FPDF_PAGEOBJECT, matrix: *const FS_MATRIX) -> FPDF_BOOL,
    extern_FPDFPageObj_NewImageObj: unsafe extern "C" fn(document: FPDF_DOCUMENT) -> FPDF_PAGEOBJECT,
    extern_FPDFPageObj_GetMarkedContentID: unsafe extern "C" fn(page_object: FPDF_PAGEOBJECT) -> c_int,
    extern_FPDFPageObj_CountMarks: unsafe extern "C" fn(page_object: FPDF_PAGEOBJECT) -> c_int,
    extern_FPDFPageObj_GetMark:
        unsafe extern "C" fn(page_object: FPDF_PAGEOBJECT, index: c_ulong) -> FPDF_PAGEOBJECTMARK,
    extern_FPDFPageObj_AddMark:
        unsafe extern "C" fn(page_object: FPDF_PAGEOBJECT, name: FPDF_BYTESTRING) -> FPDF_PAGEOBJECTMARK,
    extern_FPDFPageObj_RemoveMark:
        unsafe extern "C" fn(page_object: FPDF_PAGEOBJECT, mark: FPDF_PAGEOBJECTMARK) -> FPDF_BOOL,
    extern_FPDFPageObjMark_GetName: unsafe extern "C" fn(
        mark: FPDF_PAGEOBJECTMARK,
        buffer: *mut FPDF_WCHAR,
        buflen: c_ulong,
        out_buflen: *mut c_ulong,
    ) -> FPDF_BOOL,
    extern_FPDFPageObjMark_CountParams: unsafe extern "C" fn(mark: FPDF_PAGEOBJECTMARK) -> c_int,
    extern_FPDFPageObjMark_GetParamKey: unsafe extern "C" fn(
        mark: FPDF_PAGEOBJECTMARK,
        index: c_ulong,
        buffer: *mut FPDF_WCHAR,
        buflen: c_ulong,
        out_buflen: *mut c_ulong,
    ) -> FPDF_BOOL,
    extern_FPDFPageObjMark_GetParamValueType:
        unsafe extern "C" fn(mark: FPDF_PAGEOBJECTMARK, key: FPDF_BYTESTRING) -> FPDF_OBJECT_TYPE,
    extern_FPDFPageObjMark_GetParamIntValue:
        unsafe extern "C" fn(mark: FPDF_PAGEOBJECTMARK, key: FPDF_BYTESTRING, out_value: *mut c_int) -> FPDF_BOOL,
    extern_FPDFPageObjMark_GetParamFloatValue:
        unsafe extern "C" fn(mark: FPDF_PAGEOBJECTMARK, key: FPDF_BYTESTRING, out_value: *mut c_float) -> FPDF_BOOL,
    extern_FPDFPageObjMark_GetParamStringValue: unsafe extern "C" fn(
        mark: FPDF_PAGEOBJECTMARK,
        key: FPDF_BYTESTRING,
        buffer: *mut FPDF_WCHAR,
        buflen: c_ulong,
        out_buflen: *mut c_ulong,
    ) -> FPDF_BOOL,
    extern_FPDFPageObjMark_GetParamBlobValue: unsafe extern "C" fn(
        mark: FPDF_PAGEOBJECTMARK,
        key: FPDF_BYTESTRING,
        buffer: *mut c_uchar,
        buflen: c_ulong,
        out_buflen: *mut c_ulong,
    ) -> FPDF_BOOL,
    extern_FPDFPageObjMark_SetIntParam: unsafe extern "C" fn(
        document: FPDF_DOCUMENT,
        page_object: FPDF_PAGEOBJECT,
        mark: FPDF_PAGEOBJECTMARK,
        key: FPDF_BYTESTRING,
        value: c_int,
    ) -> FPDF_BOOL,
    extern_FPDFPageObjMark_SetFloatParam: unsafe extern "C" fn(
        document: FPDF_DOCUMENT,
        page_object: FPDF_PAGEOBJECT,
        mark: FPDF_PAGEOBJECTMARK,
        key: FPDF_BYTESTRING,
        value: c_float,
    ) -> FPDF_BOOL,
    extern_FPDFPageObjMark_SetStringParam: unsafe extern "C" fn(
        document: FPDF_DOCUMENT,
        page_object: FPDF_PAGEOBJECT,
        mark: FPDF_PAGEOBJECTMARK,
        key: FPDF_BYTESTRING,
        value: FPDF_BYTESTRING,
    ) -> FPDF_BOOL,
    extern_FPDFPageObjMark_SetBlobParam: unsafe extern "C" fn(
        document: FPDF_DOCUMENT,
        page_object: FPDF_PAGEOBJECT,
        mark: FPDF_PAGEOBJECTMARK,
        key: FPDF_BYTESTRING,
        value: *const c_uchar,
        value_len: c_ulong,
    ) -> FPDF_BOOL,
    extern_FPDFPageObjMark_RemoveParam: unsafe extern "C" fn(
        page_object: FPDF_PAGEOBJECT,
        mark: FPDF_PAGEOBJECTMARK,
        key: FPDF_BYTESTRING,
    ) -> FPDF_BOOL,
    extern_FPDFImageObj_LoadJpegFile: unsafe extern "C" fn(
        pages: *mut FPDF_PAGE,
        count: c_int,
        image_object: FPDF_PAGEOBJECT,
        file_access: *mut FPDF_FILEACCESS,
    ) -> FPDF_BOOL,
    extern_FPDFImageObj_LoadJpegFileInline: unsafe extern "C" fn(
        pages: *mut FPDF_PAGE,
        count: c_int,
        image_object: FPDF_PAGEOBJECT,
        file_access: *mut FPDF_FILEACCESS,
    ) -> FPDF_BOOL,
    extern_FPDFImageObj_SetMatrix: unsafe extern "C" fn(
        image_object: FPDF_PAGEOBJECT,
        a: c_double,
        b: c_double,
        c: c_double,
        d: c_double,
        e: c_double,
        f: c_double,
    ) -> FPDF_BOOL,
    extern_FPDFImageObj_SetBitmap: unsafe extern "C" fn(
        pages: *mut FPDF_PAGE,
        count: c_int,
        image_object: FPDF_PAGEOBJECT,
        bitmap: FPDF_BITMAP,
    ) -> FPDF_BOOL,
    extern_FPDFImageObj_GetBitmap: unsafe extern "C" fn(image_object: FPDF_PAGEOBJECT) -> FPDF_BITMAP,
    extern_FPDFImageObj_GetRenderedBitmap:
        unsafe extern "C" fn(document: FPDF_DOCUMENT, page: FPDF_PAGE, image_object: FPDF_PAGEOBJECT) -> FPDF_BITMAP,
    extern_FPDFImageObj_GetImageDataDecoded:
        unsafe extern "C" fn(image_object: FPDF_PAGEOBJECT, buffer: *mut c_void, buflen: c_ulong) -> c_ulong,
    extern_FPDFImageObj_GetImageDataRaw:
        unsafe extern "C" fn(image_object: FPDF_PAGEOBJECT, buffer: *mut c_void, buflen: c_ulong) -> c_ulong,
    extern_FPDFImageObj_GetImageFilterCount: unsafe extern "C" fn(image_object: FPDF_PAGEOBJECT) -> c_int,
    extern_FPDFImageObj_GetImageFilter: unsafe extern "C" fn(
        image_object: FPDF_PAGEOBJECT,
        index: c_int,
        buffer: *mut c_void,
        buflen: c_ulong,
    ) -> c_ulong,
    extern_FPDFImageObj_GetImageMetadata: unsafe extern "C" fn(
        image_object: FPDF_PAGEOBJECT,
        page: FPDF_PAGE,
        metadata: *mut FPDF_IMAGEOBJ_METADATA,
    ) -> FPDF_BOOL,
    extern_FPDFImageObj_GetImagePixelSize:
        unsafe extern "C" fn(image_object: FPDF_PAGEOBJECT, width: *mut c_uint, height: *mut c_uint) -> FPDF_BOOL,
    extern_FPDFImageObj_GetIccProfileDataDecoded: unsafe extern "C" fn(
        image_object: FPDF_PAGEOBJECT,
        page: FPDF_PAGE,
        buffer: *mut u8,
        buflen: size_t,
        out_buflen: *mut size_t,
    ) -> FPDF_BOOL,
    extern_FPDFPageObj_CreateNewPath: unsafe extern "C" fn(x: c_float, y: c_float) -> FPDF_PAGEOBJECT,
    extern_FPDFPageObj_CreateNewRect:
        unsafe extern "C" fn(x: c_float, y: c_float, w: c_float, h: c_float) -> FPDF_PAGEOBJECT,
    extern_FPDFPageObj_GetBounds: unsafe extern "C" fn(
        page_object: FPDF_PAGEOBJECT,
        left: *mut c_float,
        bottom: *mut c_float,
        right: *mut c_float,
        top: *mut c_float,
    ) -> FPDF_BOOL,
    extern_FPDFPageObj_GetRotatedBounds:
        unsafe extern "C" fn(page_object: FPDF_PAGEOBJECT, quad_points: *mut FS_QUADPOINTSF) -> FPDF_BOOL,
    extern_FPDFPageObj_SetBlendMode: unsafe extern "C" fn(page_object: FPDF_PAGEOBJECT, blend_mode: FPDF_BYTESTRING),
    extern_FPDFPageObj_SetStrokeColor:
        unsafe extern "C" fn(page_object: FPDF_PAGEOBJECT, R: c_uint, G: c_uint, B: c_uint, A: c_uint) -> FPDF_BOOL,
    extern_FPDFPageObj_GetStrokeColor: unsafe extern "C" fn(
        page_object: FPDF_PAGEOBJECT,
        R: *mut c_uint,
        G: *mut c_uint,
        B: *mut c_uint,
        A: *mut c_uint,
    ) -> FPDF_BOOL,
    extern_FPDFPageObj_SetStrokeWidth: unsafe extern "C" fn(page_object: FPDF_PAGEOBJECT, width: c_float) -> FPDF_BOOL,
    extern_FPDFPageObj_GetStrokeWidth:
        unsafe extern "C" fn(page_object: FPDF_PAGEOBJECT, width: *mut c_float) -> FPDF_BOOL,
    extern_FPDFPageObj_GetLineJoin: unsafe extern "C" fn(page_object: FPDF_PAGEOBJECT) -> c_int,
    extern_FPDFPageObj_SetLineJoin: unsafe extern "C" fn(page_object: FPDF_PAGEOBJECT, line_join: c_int) -> FPDF_BOOL,
    extern_FPDFPageObj_GetLineCap: unsafe extern "C" fn(page_object: FPDF_PAGEOBJECT) -> c_int,
    extern_FPDFPageObj_SetLineCap: unsafe extern "C" fn(page_object: FPDF_PAGEOBJECT, line_cap: c_int) -> FPDF_BOOL,
    extern_FPDFPageObj_SetFillColor:
        unsafe extern "C" fn(page_object: FPDF_PAGEOBJECT, R: c_uint, G: c_uint, B: c_uint, A: c_uint) -> FPDF_BOOL,
    extern_FPDFPageObj_GetFillColor: unsafe extern "C" fn(
        page_object: FPDF_PAGEOBJECT,
        R: *mut c_uint,
        G: *mut c_uint,
        B: *mut c_uint,
        A: *mut c_uint,
    ) -> FPDF_BOOL,
    extern_FPDFPageObj_GetDashPhase:
        unsafe extern "C" fn(page_object: FPDF_PAGEOBJECT, phase: *mut c_float) -> FPDF_BOOL,
    extern_FPDFPageObj_SetDashPhase: unsafe extern "C" fn(page_object: FPDF_PAGEOBJECT, phase: c_float) -> FPDF_BOOL,
    extern_FPDFPageObj_GetDashCount: unsafe extern "C" fn(page_object: FPDF_PAGEOBJECT) -> c_int,
    extern_FPDFPageObj_GetDashArray:
        unsafe extern "C" fn(page_object: FPDF_PAGEOBJECT, dash_array: *mut c_float, dash_count: size_t) -> FPDF_BOOL,
    extern_FPDFPageObj_SetDashArray: unsafe extern "C" fn(
        page_object: FPDF_PAGEOBJECT,
        dash_array: *const c_float,
        dash_count: size_t,
        phase: c_float,
    ) -> FPDF_BOOL,
    extern_FPDFPath_CountSegments: unsafe extern "C" fn(path: FPDF_PAGEOBJECT) -> c_int,
    extern_FPDFPath_GetPathSegment: unsafe extern "C" fn(path: FPDF_PAGEOBJECT, index: c_int) -> FPDF_PATHSEGMENT,
    extern_FPDFPathSegment_GetPoint:
        unsafe extern "C" fn(segment: FPDF_PATHSEGMENT, x: *mut f32, y: *mut f32) -> FPDF_BOOL,
    extern_FPDFPathSegment_GetType: unsafe extern "C" fn(segment: FPDF_PATHSEGMENT) -> c_int,
    extern_FPDFPathSegment_GetClose: unsafe extern "C" fn(segment: FPDF_PATHSEGMENT) -> FPDF_BOOL,
    extern_FPDFFont_GetBaseFontName: unsafe extern "C" fn(font: FPDF_FONT, buffer: *mut c_char, length: usize) -> usize,
    extern_FPDFFont_GetFamilyName: unsafe extern "C" fn(font: FPDF_FONT, buffer: *mut c_char, length: usize) -> usize,
    extern_FPDFFont_GetFontData:
        unsafe extern "C" fn(font: FPDF_FONT, buffer: *mut u8, buflen: usize, out_buflen: *mut usize) -> FPDF_BOOL,
    extern_FPDFFont_GetIsEmbedded: unsafe extern "C" fn(font: FPDF_FONT) -> c_int,
    extern_FPDFFont_GetFlags: unsafe extern "C" fn(font: FPDF_FONT) -> c_int,
    extern_FPDFFont_GetWeight: unsafe extern "C" fn(font: FPDF_FONT) -> c_int,
    extern_FPDFFont_GetItalicAngle: unsafe extern "C" fn(font: FPDF_FONT, angle: *mut c_int) -> FPDF_BOOL,
    extern_FPDFFont_GetAscent:
        unsafe extern "C" fn(font: FPDF_FONT, font_size: c_float, ascent: *mut c_float) -> FPDF_BOOL,
    extern_FPDFFont_GetDescent:
        unsafe extern "C" fn(font: FPDF_FONT, font_size: c_float, descent: *mut c_float) -> FPDF_BOOL,
    extern_FPDFFont_GetGlyphWidth:
        unsafe extern "C" fn(font: FPDF_FONT, glyph: c_uint, font_size: c_float, width: *mut c_float) -> FPDF_BOOL,
    extern_FPDFFont_GetGlyphPath:
        unsafe extern "C" fn(font: FPDF_FONT, glyph: c_uint, font_size: c_float) -> FPDF_GLYPHPATH,
    extern_FPDFGlyphPath_CountGlyphSegments: unsafe extern "C" fn(glyphpath: FPDF_GLYPHPATH) -> c_int,
    extern_FPDFGlyphPath_GetGlyphPathSegment:
        unsafe extern "C" fn(glyphpath: FPDF_GLYPHPATH, index: c_int) -> FPDF_PATHSEGMENT,
    extern_FPDF_VIEWERREF_GetPrintScaling: unsafe extern "C" fn(document: FPDF_DOCUMENT) -> FPDF_BOOL,
    extern_FPDF_VIEWERREF_GetNumCopies: unsafe extern "C" fn(document: FPDF_DOCUMENT) -> c_int,
    extern_FPDF_VIEWERREF_GetPrintPageRange: unsafe extern "C" fn(document: FPDF_DOCUMENT) -> FPDF_PAGERANGE,
    extern_FPDF_VIEWERREF_GetPrintPageRangeCount: unsafe extern "C" fn(pagerange: FPDF_PAGERANGE) -> size_t,
    extern_FPDF_VIEWERREF_GetPrintPageRangeElement:
        unsafe extern "C" fn(pagerange: FPDF_PAGERANGE, index: size_t) -> c_int,
    extern_FPDF_VIEWERREF_GetDuplex: unsafe extern "C" fn(document: FPDF_DOCUMENT) -> FPDF_DUPLEXTYPE,
    extern_FPDF_VIEWERREF_GetName: unsafe extern "C" fn(
        document: FPDF_DOCUMENT,
        key: FPDF_BYTESTRING,
        buffer: *mut c_char,
        length: c_ulong,
    ) -> c_ulong,
    extern_FPDF_CountNamedDests: unsafe extern "C" fn(document: FPDF_DOCUMENT) -> FPDF_DWORD,
    extern_FPDF_GetNamedDestByName: unsafe extern "C" fn(document: FPDF_DOCUMENT, name: FPDF_BYTESTRING) -> FPDF_DEST,
    extern_FPDF_GetNamedDest: unsafe extern "C" fn(
        document: FPDF_DOCUMENT,
        index: c_int,
        buffer: *mut c_void,
        buflen: *mut c_long,
    ) -> FPDF_DEST,

    extern_FPDFDoc_GetAttachmentCount: unsafe extern "C" fn(document: FPDF_DOCUMENT) -> c_int,
    extern_FPDFDoc_AddAttachment:
        unsafe extern "C" fn(document: FPDF_DOCUMENT, name: FPDF_WIDESTRING) -> FPDF_ATTACHMENT,
    extern_FPDFDoc_GetAttachment: unsafe extern "C" fn(document: FPDF_DOCUMENT, index: c_int) -> FPDF_ATTACHMENT,
    extern_FPDFDoc_DeleteAttachment: unsafe extern "C" fn(document: FPDF_DOCUMENT, index: c_int) -> FPDF_BOOL,
    extern_FPDFAttachment_GetName:
        unsafe extern "C" fn(attachment: FPDF_ATTACHMENT, buffer: *mut FPDF_WCHAR, buflen: c_ulong) -> c_ulong,
    extern_FPDFAttachment_HasKey: unsafe extern "C" fn(attachment: FPDF_ATTACHMENT, key: FPDF_BYTESTRING) -> FPDF_BOOL,
    extern_FPDFAttachment_GetValueType:
        unsafe extern "C" fn(attachment: FPDF_ATTACHMENT, key: FPDF_BYTESTRING) -> FPDF_OBJECT_TYPE,
    extern_FPDFAttachment_SetStringValue:
        unsafe extern "C" fn(attachment: FPDF_ATTACHMENT, key: FPDF_BYTESTRING, value: FPDF_WIDESTRING) -> FPDF_BOOL,
    extern_FPDFAttachment_GetStringValue: unsafe extern "C" fn(
        attachment: FPDF_ATTACHMENT,
        key: FPDF_BYTESTRING,
        buffer: *mut FPDF_WCHAR,
        buflen: c_ulong,
    ) -> c_ulong,
    extern_FPDFAttachment_SetFile: unsafe extern "C" fn(
        attachment: FPDF_ATTACHMENT,
        document: FPDF_DOCUMENT,
        contents: *const c_void,
        len: c_ulong,
    ) -> FPDF_BOOL,
    extern_FPDFAttachment_GetFile: unsafe extern "C" fn(
        attachment: FPDF_ATTACHMENT,
        buffer: *mut c_void,
        buflen: c_ulong,
        out_buflen: *mut c_ulong,
    ) -> FPDF_BOOL,
    extern_FPDFAttachment_GetSubtype:
        unsafe extern "C" fn(attachment: FPDF_ATTACHMENT, buffer: *mut FPDF_WCHAR, buflen: c_ulong) -> c_ulong,
    extern_FPDFCatalog_IsTagged: unsafe extern "C" fn(document: FPDF_DOCUMENT) -> FPDF_BOOL,
    extern_FPDFCatalog_SetLanguage:
        unsafe extern "C" fn(document: FPDF_DOCUMENT, language: FPDF_BYTESTRING) -> FPDF_BOOL,
}

impl DynamicPdfiumBindings {
    fn bind<'a, T>(library: &'a Library, function: &str) -> Result<Symbol<'a, T>, PdfiumError> {
        let c_function = CString::new(function).map_err(|err| {
            PdfiumError::LoadLibraryFunctionNameError(format!(
                "Error converting function name to CString: {function}, message: {err}"
            ))
        })?;

        unsafe {
            library
                .get(c_function.as_bytes_with_nul())
                .map_err(PdfiumError::LoadLibraryError)
        }
    }

    pub fn new(library: Library) -> Result<Self, PdfiumError> {
        Ok(DynamicPdfiumBindings {
            extern_FPDF_InitLibraryWithConfig: *(Self::bind(&library, "FPDF_InitLibraryWithConfig")?),
            extern_FPDF_InitLibrary: *(Self::bind(&library, "FPDF_InitLibrary")?),
            extern_FPDF_SetSandBoxPolicy: *(Self::bind(&library, "FPDF_SetSandBoxPolicy")?),
            extern_FPDF_DestroyLibrary: *(Self::bind(&library, "FPDF_DestroyLibrary")?),
            extern_FPDF_GetLastError: *(Self::bind(&library, "FPDF_GetLastError")?),
            extern_FPDF_CreateNewDocument: *(Self::bind(&library, "FPDF_CreateNewDocument")?),
            extern_FPDF_LoadDocument: *(Self::bind(&library, "FPDF_LoadDocument")?),
            extern_FPDF_LoadMemDocument64: *(Self::bind(&library, "FPDF_LoadMemDocument64")?),
            extern_FPDF_LoadCustomDocument: *(Self::bind(&library, "FPDF_LoadCustomDocument")?),
            extern_FPDF_SaveAsCopy: *(Self::bind(&library, "FPDF_SaveAsCopy")?),
            extern_FPDF_SaveWithVersion: *(Self::bind(&library, "FPDF_SaveWithVersion")?),
            extern_FPDFAvail_Create: *(Self::bind(&library, "FPDFAvail_Create")?),
            extern_FPDFAvail_Destroy: *(Self::bind(&library, "FPDFAvail_Destroy")?),
            extern_FPDFAvail_IsDocAvail: *(Self::bind(&library, "FPDFAvail_IsDocAvail")?),
            extern_FPDFAvail_GetDocument: *(Self::bind(&library, "FPDFAvail_GetDocument")?),
            extern_FPDFAvail_GetFirstPageNum: *(Self::bind(&library, "FPDFAvail_GetFirstPageNum")?),
            extern_FPDFAvail_IsPageAvail: *(Self::bind(&library, "FPDFAvail_IsPageAvail")?),
            extern_FPDFAvail_IsFormAvail: *(Self::bind(&library, "FPDFAvail_IsFormAvail")?),
            extern_FPDFAvail_IsLinearized: *(Self::bind(&library, "FPDFAvail_IsLinearized")?),
            extern_FPDF_CloseDocument: *(Self::bind(&library, "FPDF_CloseDocument")?),
            extern_FPDF_DeviceToPage: *(Self::bind(&library, "FPDF_DeviceToPage")?),
            extern_FPDF_PageToDevice: *(Self::bind(&library, "FPDF_PageToDevice")?),
            extern_FPDF_GetFileVersion: *(Self::bind(&library, "FPDF_GetFileVersion")?),
            extern_FPDF_GetFileIdentifier: *(Self::bind(&library, "FPDF_GetFileIdentifier")?),
            extern_FPDF_GetMetaText: *(Self::bind(&library, "FPDF_GetMetaText")?),
            extern_FPDF_DocumentHasValidCrossReferenceTable: *(Self::bind(
                &library,
                "FPDF_DocumentHasValidCrossReferenceTable",
            )?),
            extern_FPDF_GetTrailerEnds: *(Self::bind(&library, "FPDF_GetTrailerEnds")?),
            extern_FPDF_GetDocPermissions: *(Self::bind(&library, "FPDF_GetDocPermissions")?),
            extern_FPDF_GetDocUserPermissions: *(Self::bind(&library, "FPDF_GetDocUserPermissions")?),
            extern_FPDF_GetSecurityHandlerRevision: *(Self::bind(&library, "FPDF_GetSecurityHandlerRevision")?),
            extern_FPDF_GetPageCount: *(Self::bind(&library, "FPDF_GetPageCount")?),
            extern_FPDF_LoadPage: *(Self::bind(&library, "FPDF_LoadPage")?),
            extern_FPDF_ClosePage: *(Self::bind(&library, "FPDF_ClosePage")?),
            extern_FPDF_RenderPageBitmapWithColorScheme_Start: *(Self::bind(
                &library,
                "FPDF_RenderPageBitmapWithColorScheme_Start",
            )?),
            extern_FPDF_RenderPageBitmap_Start: *(Self::bind(&library, "FPDF_RenderPageBitmap_Start")?),
            extern_FPDF_RenderPage_Continue: *(Self::bind(&library, "FPDF_RenderPage_Continue")?),
            extern_FPDF_RenderPage_Close: *(Self::bind(&library, "FPDF_RenderPage_Close")?),
            extern_FPDF_ImportPagesByIndex: *(Self::bind(&library, "FPDF_ImportPagesByIndex")?),
            extern_FPDF_ImportPages: *(Self::bind(&library, "FPDF_ImportPages")?),
            extern_FPDF_ImportNPagesToOne: *(Self::bind(&library, "FPDF_ImportNPagesToOne")?),
            extern_FPDF_GetPageLabel: *(Self::bind(&library, "FPDF_GetPageLabel")?),
            extern_FPDF_GetPageBoundingBox: *(Self::bind(&library, "FPDF_GetPageBoundingBox")?),
            extern_FPDF_GetPageSizeByIndexF: *(Self::bind(&library, "FPDF_GetPageSizeByIndexF")?),
            extern_FPDF_GetPageSizeByIndex: *(Self::bind(&library, "FPDF_GetPageSizeByIndex")?),
            extern_FPDF_NewXObjectFromPage: *(Self::bind(&library, "FPDF_NewXObjectFromPage")?),
            extern_FPDF_CloseXObject: *(Self::bind(&library, "FPDF_CloseXObject")?),
            extern_FPDF_NewFormObjectFromXObject: *(Self::bind(&library, "FPDF_NewFormObjectFromXObject")?),
            extern_FPDF_CopyViewerPreferences: *(Self::bind(&library, "FPDF_CopyViewerPreferences")?),
            extern_FPDF_GetPageWidth: *(Self::bind(&library, "FPDF_GetPageWidth")?),
            extern_FPDF_GetPageHeight: *(Self::bind(&library, "FPDF_GetPageHeight")?),
            extern_FPDF_GetPageWidthF: *(Self::bind(&library, "FPDF_GetPageWidthF")?),
            extern_FPDF_GetPageHeightF: *(Self::bind(&library, "FPDF_GetPageHeightF")?),
            extern_FPDFText_GetCharIndexFromTextIndex: *(Self::bind(&library, "FPDFText_GetCharIndexFromTextIndex")?),
            extern_FPDFText_GetTextIndexFromCharIndex: *(Self::bind(&library, "FPDFText_GetTextIndexFromCharIndex")?),
            extern_FPDF_GetSignatureCount: *(Self::bind(&library, "FPDF_GetSignatureCount")?),
            extern_FPDF_GetSignatureObject: *(Self::bind(&library, "FPDF_GetSignatureObject")?),
            extern_FPDFSignatureObj_GetContents: *(Self::bind(&library, "FPDFSignatureObj_GetContents")?),
            extern_FPDFSignatureObj_GetByteRange: *(Self::bind(&library, "FPDFSignatureObj_GetByteRange")?),
            extern_FPDFSignatureObj_GetSubFilter: *(Self::bind(&library, "FPDFSignatureObj_GetSubFilter")?),
            extern_FPDFSignatureObj_GetReason: *(Self::bind(&library, "FPDFSignatureObj_GetReason")?),
            extern_FPDFSignatureObj_GetTime: *(Self::bind(&library, "FPDFSignatureObj_GetTime")?),
            extern_FPDFSignatureObj_GetDocMDPPermission: *(Self::bind(
                &library,
                "FPDFSignatureObj_GetDocMDPPermission",
            )?),
            extern_FPDF_StructTree_GetForPage: *(Self::bind(&library, "FPDF_StructTree_GetForPage")?),
            extern_FPDF_StructTree_Close: *(Self::bind(&library, "FPDF_StructTree_Close")?),
            extern_FPDF_StructTree_CountChildren: *(Self::bind(&library, "FPDF_StructTree_CountChildren")?),
            extern_FPDF_StructTree_GetChildAtIndex: *(Self::bind(&library, "FPDF_StructTree_GetChildAtIndex")?),
            extern_FPDF_StructElement_GetAltText: *(Self::bind(&library, "FPDF_StructElement_GetAltText")?),
            extern_FPDF_StructElement_GetID: *(Self::bind(&library, "FPDF_StructElement_GetID")?),
            extern_FPDF_StructElement_GetLang: *(Self::bind(&library, "FPDF_StructElement_GetLang")?),
            extern_FPDF_StructElement_GetStringAttribute: *(Self::bind(
                &library,
                "FPDF_StructElement_GetStringAttribute",
            )?),
            extern_FPDF_StructElement_GetMarkedContentID: *(Self::bind(
                &library,
                "FPDF_StructElement_GetMarkedContentID",
            )?),
            extern_FPDF_StructElement_GetType: *(Self::bind(&library, "FPDF_StructElement_GetType")?),
            extern_FPDF_StructElement_GetTitle: *(Self::bind(&library, "FPDF_StructElement_GetTitle")?),
            extern_FPDF_StructElement_CountChildren: *(Self::bind(&library, "FPDF_StructElement_CountChildren")?),
            extern_FPDF_StructElement_GetChildAtIndex: *(Self::bind(&library, "FPDF_StructElement_GetChildAtIndex")?),
            extern_FPDF_StructElement_GetActualText: *(Self::bind(&library, "FPDF_StructElement_GetActualText")?),
            extern_FPDF_StructElement_GetObjType: *(Self::bind(&library, "FPDF_StructElement_GetObjType")?),
            extern_FPDF_StructElement_GetChildMarkedContentID: *(Self::bind(
                &library,
                "FPDF_StructElement_GetChildMarkedContentID",
            )?),
            extern_FPDF_StructElement_GetParent: *(Self::bind(&library, "FPDF_StructElement_GetParent")?),
            extern_FPDF_StructElement_GetAttributeCount: *(Self::bind(
                &library,
                "FPDF_StructElement_GetAttributeCount",
            )?),
            extern_FPDF_StructElement_GetAttributeAtIndex: *(Self::bind(
                &library,
                "FPDF_StructElement_GetAttributeAtIndex",
            )?),
            extern_FPDF_StructElement_Attr_GetCount: *(Self::bind(&library, "FPDF_StructElement_Attr_GetCount")?),
            extern_FPDF_StructElement_Attr_GetName: *(Self::bind(&library, "FPDF_StructElement_Attr_GetName")?),
            extern_FPDF_StructElement_Attr_GetValue: *(Self::bind(&library, "FPDF_StructElement_Attr_GetValue")?),
            extern_FPDF_StructElement_Attr_GetType: *(Self::bind(&library, "FPDF_StructElement_Attr_GetType")?),
            extern_FPDF_StructElement_Attr_GetBooleanValue: *(Self::bind(
                &library,
                "FPDF_StructElement_Attr_GetBooleanValue",
            )?),
            extern_FPDF_StructElement_Attr_GetNumberValue: *(Self::bind(
                &library,
                "FPDF_StructElement_Attr_GetNumberValue",
            )?),
            extern_FPDF_StructElement_Attr_GetStringValue: *(Self::bind(
                &library,
                "FPDF_StructElement_Attr_GetStringValue",
            )?),
            extern_FPDF_StructElement_Attr_GetBlobValue: *(Self::bind(
                &library,
                "FPDF_StructElement_Attr_GetBlobValue",
            )?),
            extern_FPDF_StructElement_Attr_CountChildren: *(Self::bind(
                &library,
                "FPDF_StructElement_Attr_CountChildren",
            )?),
            extern_FPDF_StructElement_Attr_GetChildAtIndex: *(Self::bind(
                &library,
                "FPDF_StructElement_Attr_GetChildAtIndex",
            )?),
            extern_FPDF_StructElement_GetMarkedContentIdCount: *(Self::bind(
                &library,
                "FPDF_StructElement_GetMarkedContentIdCount",
            )?),
            extern_FPDF_StructElement_GetMarkedContentIdAtIndex: *(Self::bind(
                &library,
                "FPDF_StructElement_GetMarkedContentIdAtIndex",
            )?),
            extern_FPDFPage_New: *(Self::bind(&library, "FPDFPage_New")?),
            extern_FPDFPage_Delete: *(Self::bind(&library, "FPDFPage_Delete")?),
            extern_FPDF_MovePages: *(Self::bind(&library, "FPDF_MovePages")?),
            extern_FPDFPage_GetRotation: *(Self::bind(&library, "FPDFPage_GetRotation")?),
            extern_FPDFPage_SetRotation: *(Self::bind(&library, "FPDFPage_SetRotation")?),
            extern_FPDFPage_GetMediaBox: *(Self::bind(&library, "FPDFPage_GetMediaBox")?),
            extern_FPDFPage_GetCropBox: *(Self::bind(&library, "FPDFPage_GetCropBox")?),
            extern_FPDFPage_GetBleedBox: *(Self::bind(&library, "FPDFPage_GetBleedBox")?),
            extern_FPDFPage_GetTrimBox: *(Self::bind(&library, "FPDFPage_GetTrimBox")?),
            extern_FPDFPage_GetArtBox: *(Self::bind(&library, "FPDFPage_GetArtBox")?),
            extern_FPDFPage_SetMediaBox: *(Self::bind(&library, "FPDFPage_SetMediaBox")?),
            extern_FPDFPage_SetCropBox: *(Self::bind(&library, "FPDFPage_SetCropBox")?),
            extern_FPDFPage_SetBleedBox: *(Self::bind(&library, "FPDFPage_SetBleedBox")?),
            extern_FPDFPage_SetTrimBox: *(Self::bind(&library, "FPDFPage_SetTrimBox")?),
            extern_FPDFPage_SetArtBox: *(Self::bind(&library, "FPDFPage_SetArtBox")?),
            extern_FPDFPage_TransFormWithClip: *(Self::bind(&library, "FPDFPage_TransFormWithClip")?),
            extern_FPDFPageObj_TransformClipPath: *(Self::bind(&library, "FPDFPageObj_TransformClipPath")?),
            extern_FPDFPageObj_GetClipPath: *(Self::bind(&library, "FPDFPageObj_GetClipPath")?),
            extern_FPDFClipPath_CountPaths: *(Self::bind(&library, "FPDFClipPath_CountPaths")?),
            extern_FPDFClipPath_CountPathSegments: *(Self::bind(&library, "FPDFClipPath_CountPathSegments")?),
            extern_FPDFClipPath_GetPathSegment: *(Self::bind(&library, "FPDFClipPath_GetPathSegment")?),
            extern_FPDF_CreateClipPath: *(Self::bind(&library, "FPDF_CreateClipPath")?),
            extern_FPDF_DestroyClipPath: *(Self::bind(&library, "FPDF_DestroyClipPath")?),
            extern_FPDFPage_InsertClipPath: *(Self::bind(&library, "FPDFPage_InsertClipPath")?),
            extern_FPDFPage_HasTransparency: *(Self::bind(&library, "FPDFPage_HasTransparency")?),
            extern_FPDFPage_GenerateContent: *(Self::bind(&library, "FPDFPage_GenerateContent")?),
            extern_FPDFPage_TransformAnnots: *(Self::bind(&library, "FPDFPage_TransformAnnots")?),
            extern_FPDFBitmap_Create: *(Self::bind(&library, "FPDFBitmap_Create")?),
            extern_FPDFBitmap_CreateEx: *(Self::bind(&library, "FPDFBitmap_CreateEx")?),
            extern_FPDFBitmap_Destroy: *(Self::bind(&library, "FPDFBitmap_Destroy")?),
            extern_FPDFBitmap_GetFormat: *(Self::bind(&library, "FPDFBitmap_GetFormat")?),
            extern_FPDFBitmap_FillRect: *(Self::bind(&library, "FPDFBitmap_FillRect")?),
            extern_FPDFBitmap_GetBuffer: *(Self::bind(&library, "FPDFBitmap_GetBuffer")?),
            extern_FPDFBitmap_GetWidth: *(Self::bind(&library, "FPDFBitmap_GetWidth")?),
            extern_FPDFBitmap_GetHeight: *(Self::bind(&library, "FPDFBitmap_GetHeight")?),
            extern_FPDFBitmap_GetStride: *(Self::bind(&library, "FPDFBitmap_GetStride")?),
            extern_FPDF_RenderPageBitmap: *(Self::bind(&library, "FPDF_RenderPageBitmap")?),
            extern_FPDF_RenderPageBitmapWithMatrix: *(Self::bind(&library, "FPDF_RenderPageBitmapWithMatrix")?),
            extern_FPDFAnnot_IsSupportedSubtype: *(Self::bind(&library, "FPDFAnnot_IsSupportedSubtype")?),
            extern_FPDFPage_CreateAnnot: *(Self::bind(&library, "FPDFPage_CreateAnnot")?),
            extern_FPDFPage_GetAnnotCount: *(Self::bind(&library, "FPDFPage_GetAnnotCount")?),
            extern_FPDFPage_GetAnnot: *(Self::bind(&library, "FPDFPage_GetAnnot")?),
            extern_FPDFPage_GetAnnotIndex: *(Self::bind(&library, "FPDFPage_GetAnnotIndex")?),
            extern_FPDFPage_CloseAnnot: *(Self::bind(&library, "FPDFPage_CloseAnnot")?),
            extern_FPDFPage_RemoveAnnot: *(Self::bind(&library, "FPDFPage_RemoveAnnot")?),
            extern_FPDFAnnot_GetSubtype: *(Self::bind(&library, "FPDFAnnot_GetSubtype")?),
            extern_FPDFAnnot_IsObjectSupportedSubtype: *(Self::bind(&library, "FPDFAnnot_IsObjectSupportedSubtype")?),
            extern_FPDFAnnot_UpdateObject: *(Self::bind(&library, "FPDFAnnot_UpdateObject")?),
            extern_FPDFAnnot_AddInkStroke: *(Self::bind(&library, "FPDFAnnot_AddInkStroke")?),
            extern_FPDFAnnot_RemoveInkList: *(Self::bind(&library, "FPDFAnnot_RemoveInkList")?),
            extern_FPDFAnnot_AppendObject: *(Self::bind(&library, "FPDFAnnot_AppendObject")?),
            extern_FPDFAnnot_GetObjectCount: *(Self::bind(&library, "FPDFAnnot_GetObjectCount")?),
            extern_FPDFAnnot_GetObject: *(Self::bind(&library, "FPDFAnnot_GetObject")?),
            extern_FPDFAnnot_RemoveObject: *(Self::bind(&library, "FPDFAnnot_RemoveObject")?),
            extern_FPDFAnnot_SetColor: *(Self::bind(&library, "FPDFAnnot_SetColor")?),
            extern_FPDFAnnot_GetColor: *(Self::bind(&library, "FPDFAnnot_GetColor")?),
            extern_FPDFAnnot_HasAttachmentPoints: *(Self::bind(&library, "FPDFAnnot_HasAttachmentPoints")?),
            extern_FPDFAnnot_SetAttachmentPoints: *(Self::bind(&library, "FPDFAnnot_SetAttachmentPoints")?),
            extern_FPDFAnnot_AppendAttachmentPoints: *(Self::bind(&library, "FPDFAnnot_AppendAttachmentPoints")?),
            extern_FPDFAnnot_CountAttachmentPoints: *(Self::bind(&library, "FPDFAnnot_CountAttachmentPoints")?),
            extern_FPDFAnnot_GetAttachmentPoints: *(Self::bind(&library, "FPDFAnnot_GetAttachmentPoints")?),
            extern_FPDFAnnot_SetRect: *(Self::bind(&library, "FPDFAnnot_SetRect")?),
            extern_FPDFAnnot_GetRect: *(Self::bind(&library, "FPDFAnnot_GetRect")?),
            extern_FPDFAnnot_GetVertices: *(Self::bind(&library, "FPDFAnnot_GetVertices")?),
            extern_FPDFAnnot_GetInkListCount: *(Self::bind(&library, "FPDFAnnot_GetInkListCount")?),
            extern_FPDFAnnot_GetInkListPath: *(Self::bind(&library, "FPDFAnnot_GetInkListPath")?),
            extern_FPDFAnnot_GetLine: *(Self::bind(&library, "FPDFAnnot_GetLine")?),
            extern_FPDFAnnot_SetBorder: *(Self::bind(&library, "FPDFAnnot_SetBorder")?),
            extern_FPDFAnnot_GetBorder: *(Self::bind(&library, "FPDFAnnot_GetBorder")?),
            extern_FPDFAnnot_GetFormAdditionalActionJavaScript: *(Self::bind(
                &library,
                "FPDFAnnot_GetFormAdditionalActionJavaScript",
            )?),
            extern_FPDFAnnot_GetFormFieldAlternateName: *(Self::bind(&library, "FPDFAnnot_GetFormFieldAlternateName")?),
            extern_FPDFAnnot_HasKey: *(Self::bind(&library, "FPDFAnnot_HasKey")?),
            extern_FPDFAnnot_GetValueType: *(Self::bind(&library, "FPDFAnnot_GetValueType")?),
            extern_FPDFAnnot_SetStringValue: *(Self::bind(&library, "FPDFAnnot_SetStringValue")?),
            extern_FPDFAnnot_GetStringValue: *(Self::bind(&library, "FPDFAnnot_GetStringValue")?),
            extern_FPDFAnnot_GetNumberValue: *(Self::bind(&library, "FPDFAnnot_GetNumberValue")?),
            extern_FPDFAnnot_SetAP: *(Self::bind(&library, "FPDFAnnot_SetAP")?),
            extern_FPDFAnnot_GetAP: *(Self::bind(&library, "FPDFAnnot_GetAP")?),
            extern_FPDFAnnot_GetLinkedAnnot: *(Self::bind(&library, "FPDFAnnot_GetLinkedAnnot")?),
            extern_FPDFAnnot_GetFlags: *(Self::bind(&library, "FPDFAnnot_GetFlags")?),
            extern_FPDFAnnot_SetFlags: *(Self::bind(&library, "FPDFAnnot_SetFlags")?),
            extern_FPDFAnnot_GetFormFieldFlags: *(Self::bind(&library, "FPDFAnnot_GetFormFieldFlags")?),
            extern_FPDFAnnot_SetFormFieldFlags: *(Self::bind(&library, "FPDFAnnot_SetFormFieldFlags")?),
            extern_FPDFAnnot_GetFormFieldAtPoint: *(Self::bind(&library, "FPDFAnnot_GetFormFieldAtPoint")?),
            extern_FPDFAnnot_GetFormFieldName: *(Self::bind(&library, "FPDFAnnot_GetFormFieldName")?),
            extern_FPDFAnnot_GetFormFieldType: *(Self::bind(&library, "FPDFAnnot_GetFormFieldType")?),
            extern_FPDFAnnot_GetFormFieldValue: *(Self::bind(&library, "FPDFAnnot_GetFormFieldValue")?),
            extern_FPDFAnnot_GetOptionCount: *(Self::bind(&library, "FPDFAnnot_GetOptionCount")?),
            extern_FPDFAnnot_GetOptionLabel: *(Self::bind(&library, "FPDFAnnot_GetOptionLabel")?),
            extern_FPDFAnnot_IsOptionSelected: *(Self::bind(&library, "FPDFAnnot_IsOptionSelected")?),
            extern_FPDFAnnot_GetFontSize: *(Self::bind(&library, "FPDFAnnot_GetFontSize")?),
            extern_FPDFAnnot_SetFontColor: *(Self::bind(&library, "FPDFAnnot_SetFontColor")?),
            extern_FPDFAnnot_GetFontColor: *(Self::bind(&library, "FPDFAnnot_GetFontColor")?),
            extern_FPDFAnnot_IsChecked: *(Self::bind(&library, "FPDFAnnot_IsChecked")?),
            extern_FPDFAnnot_SetFocusableSubtypes: *(Self::bind(&library, "FPDFAnnot_SetFocusableSubtypes")?),
            extern_FPDFAnnot_GetFocusableSubtypesCount: *(Self::bind(&library, "FPDFAnnot_GetFocusableSubtypesCount")?),
            extern_FPDFAnnot_GetFocusableSubtypes: *(Self::bind(&library, "FPDFAnnot_GetFocusableSubtypes")?),
            extern_FPDFAnnot_GetLink: *(Self::bind(&library, "FPDFAnnot_GetLink")?),
            extern_FPDFAnnot_GetFormControlCount: *(Self::bind(&library, "FPDFAnnot_GetFormControlCount")?),
            extern_FPDFAnnot_GetFormControlIndex: *(Self::bind(&library, "FPDFAnnot_GetFormControlIndex")?),
            extern_FPDFAnnot_GetFormFieldExportValue: *(Self::bind(&library, "FPDFAnnot_GetFormFieldExportValue")?),
            extern_FPDFAnnot_SetURI: *(Self::bind(&library, "FPDFAnnot_SetURI")?),
            extern_FPDFAnnot_GetFileAttachment: *(Self::bind(&library, "FPDFAnnot_GetFileAttachment")?),
            extern_FPDFAnnot_AddFileAttachment: *(Self::bind(&library, "FPDFAnnot_AddFileAttachment")?),
            extern_FPDFDOC_InitFormFillEnvironment: *(Self::bind(&library, "FPDFDOC_InitFormFillEnvironment")?),
            extern_FPDFDOC_ExitFormFillEnvironment: *(Self::bind(&library, "FPDFDOC_ExitFormFillEnvironment")?),
            extern_FORM_OnAfterLoadPage: *(Self::bind(&library, "FORM_OnAfterLoadPage")?),
            extern_FORM_OnBeforeClosePage: *(Self::bind(&library, "FORM_OnBeforeClosePage")?),
            extern_FPDFDoc_GetPageMode: *(Self::bind(&library, "FPDFDoc_GetPageMode")?),
            extern_FPDFPage_Flatten: *(Self::bind(&library, "FPDFPage_Flatten")?),
            extern_FORM_DoDocumentJSAction: *(Self::bind(&library, "FORM_DoDocumentJSAction")?),
            extern_FORM_DoDocumentOpenAction: *(Self::bind(&library, "FORM_DoDocumentOpenAction")?),
            extern_FORM_DoDocumentAAction: *(Self::bind(&library, "FORM_DoDocumentAAction")?),
            extern_FORM_DoPageAAction: *(Self::bind(&library, "FORM_DoPageAAction")?),
            extern_FORM_OnMouseMove: *(Self::bind(&library, "FORM_OnMouseMove")?),
            extern_FORM_OnMouseWheel: *(Self::bind(&library, "FORM_OnMouseWheel")?),
            extern_FORM_OnFocus: *(Self::bind(&library, "FORM_OnFocus")?),
            extern_FORM_OnLButtonDown: *(Self::bind(&library, "FORM_OnLButtonDown")?),
            extern_FORM_OnRButtonDown: *(Self::bind(&library, "FORM_OnRButtonDown")?),
            extern_FORM_OnLButtonUp: *(Self::bind(&library, "FORM_OnLButtonUp")?),
            extern_FORM_OnRButtonUp: *(Self::bind(&library, "FORM_OnRButtonUp")?),
            extern_FORM_OnLButtonDoubleClick: *(Self::bind(&library, "FORM_OnLButtonDoubleClick")?),
            extern_FORM_OnKeyDown: *(Self::bind(&library, "FORM_OnKeyDown")?),
            extern_FORM_OnKeyUp: *(Self::bind(&library, "FORM_OnKeyUp")?),
            extern_FORM_OnChar: *(Self::bind(&library, "FORM_OnChar")?),
            extern_FORM_GetFocusedText: *(Self::bind(&library, "FORM_GetFocusedText")?),
            extern_FORM_GetSelectedText: *(Self::bind(&library, "FORM_GetSelectedText")?),
            extern_FORM_ReplaceAndKeepSelection: *(Self::bind(&library, "FORM_ReplaceAndKeepSelection")?),
            extern_FORM_ReplaceSelection: *(Self::bind(&library, "FORM_ReplaceSelection")?),
            extern_FORM_SelectAllText: *(Self::bind(&library, "FORM_SelectAllText")?),
            extern_FORM_CanUndo: *(Self::bind(&library, "FORM_CanUndo")?),
            extern_FORM_CanRedo: *(Self::bind(&library, "FORM_CanRedo")?),
            extern_FORM_Undo: *(Self::bind(&library, "FORM_Undo")?),
            extern_FORM_Redo: *(Self::bind(&library, "FORM_Redo")?),
            extern_FORM_ForceToKillFocus: *(Self::bind(&library, "FORM_ForceToKillFocus")?),
            extern_FORM_GetFocusedAnnot: *(Self::bind(&library, "FORM_GetFocusedAnnot")?),
            extern_FORM_SetFocusedAnnot: *(Self::bind(&library, "FORM_SetFocusedAnnot")?),
            extern_FPDFPage_HasFormFieldAtPoint: *(Self::bind(&library, "FPDFPage_HasFormFieldAtPoint")?),
            extern_FPDFPage_FormFieldZOrderAtPoint: *(Self::bind(&library, "FPDFPage_FormFieldZOrderAtPoint")?),
            extern_FPDF_SetFormFieldHighlightColor: *(Self::bind(&library, "FPDF_SetFormFieldHighlightColor")?),
            extern_FPDF_SetFormFieldHighlightAlpha: *(Self::bind(&library, "FPDF_SetFormFieldHighlightAlpha")?),
            extern_FPDF_RemoveFormFieldHighlight: *(Self::bind(&library, "FPDF_RemoveFormFieldHighlight")?),
            extern_FPDF_FFLDraw: *(Self::bind(&library, "FPDF_FFLDraw")?),
            extern_FPDF_GetFormType: *(Self::bind(&library, "FPDF_GetFormType")?),
            extern_FORM_SetIndexSelected: *(Self::bind(&library, "FORM_SetIndexSelected")?),
            extern_FORM_IsIndexSelected: *(Self::bind(&library, "FORM_IsIndexSelected")?),
            extern_FPDF_LoadXFA: *(Self::bind(&library, "FPDF_LoadXFA")?),
            extern_FPDFDoc_GetJavaScriptActionCount: *(Self::bind(&library, "FPDFDoc_GetJavaScriptActionCount")?),
            extern_FPDFDoc_GetJavaScriptAction: *(Self::bind(&library, "FPDFDoc_GetJavaScriptAction")?),
            extern_FPDFDoc_CloseJavaScriptAction: *(Self::bind(&library, "FPDFDoc_CloseJavaScriptAction")?),
            extern_FPDFJavaScriptAction_GetName: *(Self::bind(&library, "FPDFJavaScriptAction_GetName")?),
            extern_FPDFJavaScriptAction_GetScript: *(Self::bind(&library, "FPDFJavaScriptAction_GetScript")?),
            extern_FPDF_GetDefaultTTFMap: *(Self::bind(&library, "FPDF_GetDefaultTTFMap")?),
            extern_FPDF_GetDefaultTTFMapCount: *(Self::bind(&library, "FPDF_GetDefaultTTFMapCount")?),
            extern_FPDF_GetDefaultTTFMapEntry: *(Self::bind(&library, "FPDF_GetDefaultTTFMapEntry")?),
            extern_FPDF_AddInstalledFont: *(Self::bind(&library, "FPDF_AddInstalledFont")?),
            extern_FPDF_SetSystemFontInfo: *(Self::bind(&library, "FPDF_SetSystemFontInfo")?),
            extern_FPDF_GetDefaultSystemFontInfo: *(Self::bind(&library, "FPDF_GetDefaultSystemFontInfo")?),
            extern_FPDF_FreeDefaultSystemFontInfo: *(Self::bind(&library, "FPDF_FreeDefaultSystemFontInfo")?),
            extern_FPDFBookmark_GetFirstChild: *(Self::bind(&library, "FPDFBookmark_GetFirstChild")?),
            extern_FPDFBookmark_GetNextSibling: *(Self::bind(&library, "FPDFBookmark_GetNextSibling")?),
            extern_FPDFBookmark_GetTitle: *(Self::bind(&library, "FPDFBookmark_GetTitle")?),
            extern_FPDFBookmark_GetCount: *(Self::bind(&library, "FPDFBookmark_GetCount")?),
            extern_FPDFBookmark_Find: *(Self::bind(&library, "FPDFBookmark_Find")?),
            extern_FPDFBookmark_GetDest: *(Self::bind(&library, "FPDFBookmark_GetDest")?),
            extern_FPDFBookmark_GetAction: *(Self::bind(&library, "FPDFBookmark_GetAction")?),
            extern_FPDFAction_GetType: *(Self::bind(&library, "FPDFAction_GetType")?),
            extern_FPDFAction_GetDest: *(Self::bind(&library, "FPDFAction_GetDest")?),
            extern_FPDFAction_GetFilePath: *(Self::bind(&library, "FPDFAction_GetFilePath")?),
            extern_FPDFAction_GetURIPath: *(Self::bind(&library, "FPDFAction_GetURIPath")?),
            extern_FPDFDest_GetDestPageIndex: *(Self::bind(&library, "FPDFDest_GetDestPageIndex")?),
            extern_FPDFDest_GetView: *(Self::bind(&library, "FPDFDest_GetView")?),
            extern_FPDFDest_GetLocationInPage: *(Self::bind(&library, "FPDFDest_GetLocationInPage")?),
            extern_FPDFLink_GetLinkAtPoint: *(Self::bind(&library, "FPDFLink_GetLinkAtPoint")?),
            extern_FPDFLink_GetLinkZOrderAtPoint: *(Self::bind(&library, "FPDFLink_GetLinkZOrderAtPoint")?),
            extern_FPDFLink_GetDest: *(Self::bind(&library, "FPDFLink_GetDest")?),
            extern_FPDFLink_GetAction: *(Self::bind(&library, "FPDFLink_GetAction")?),
            extern_FPDFLink_Enumerate: *(Self::bind(&library, "FPDFLink_Enumerate")?),
            extern_FPDFLink_GetAnnot: *(Self::bind(&library, "FPDFLink_GetAnnot")?),
            extern_FPDFLink_GetAnnotRect: *(Self::bind(&library, "FPDFLink_GetAnnotRect")?),
            extern_FPDFLink_CountQuadPoints: *(Self::bind(&library, "FPDFLink_CountQuadPoints")?),
            extern_FPDFLink_GetQuadPoints: *(Self::bind(&library, "FPDFLink_GetQuadPoints")?),
            extern_FPDF_GetPageAAction: *(Self::bind(&library, "FPDF_GetPageAAction")?),
            extern_FPDFText_LoadPage: *(Self::bind(&library, "FPDFText_LoadPage")?),
            extern_FPDFText_ClosePage: *(Self::bind(&library, "FPDFText_ClosePage")?),
            extern_FPDFText_CountChars: *(Self::bind(&library, "FPDFText_CountChars")?),
            extern_FPDFText_GetUnicode: *(Self::bind(&library, "FPDFText_GetUnicode")?),
            extern_FPDFText_GetTextObject: *(Self::bind(&library, "FPDFText_GetTextObject")?),
            extern_FPDFText_IsGenerated: *(Self::bind(&library, "FPDFText_IsGenerated")?),
            extern_FPDFText_IsHyphen: *(Self::bind(&library, "FPDFText_IsHyphen")?),
            extern_FPDFText_HasUnicodeMapError: *(Self::bind(&library, "FPDFText_HasUnicodeMapError")?),
            extern_FPDFText_GetFontSize: *(Self::bind(&library, "FPDFText_GetFontSize")?),
            extern_FPDFText_GetFontInfo: *(Self::bind(&library, "FPDFText_GetFontInfo")?),
            extern_FPDFText_GetFontWeight: *(Self::bind(&library, "FPDFText_GetFontWeight")?),
            extern_FPDFText_GetFillColor: *(Self::bind(&library, "FPDFText_GetFillColor")?),
            extern_FPDFText_GetStrokeColor: *(Self::bind(&library, "FPDFText_GetStrokeColor")?),
            extern_FPDFText_GetCharAngle: *(Self::bind(&library, "FPDFText_GetCharAngle")?),
            extern_FPDFText_GetCharBox: *(Self::bind(&library, "FPDFText_GetCharBox")?),
            extern_FPDFText_GetLooseCharBox: *(Self::bind(&library, "FPDFText_GetLooseCharBox")?),
            extern_FPDFText_GetMatrix: *(Self::bind(&library, "FPDFText_GetMatrix")?),
            extern_FPDFText_GetCharOrigin: *(Self::bind(&library, "FPDFText_GetCharOrigin")?),
            extern_FPDFText_GetCharIndexAtPos: *(Self::bind(&library, "FPDFText_GetCharIndexAtPos")?),
            extern_FPDFText_GetText: *(Self::bind(&library, "FPDFText_GetText")?),
            extern_FPDFText_CountRects: *(Self::bind(&library, "FPDFText_CountRects")?),
            extern_FPDFText_GetRect: *(Self::bind(&library, "FPDFText_GetRect")?),
            extern_FPDFText_GetBoundedText: *(Self::bind(&library, "FPDFText_GetBoundedText")?),
            extern_FPDFText_FindStart: *(Self::bind(&library, "FPDFText_FindStart")?),
            extern_FPDFText_FindNext: *(Self::bind(&library, "FPDFText_FindNext")?),
            extern_FPDFText_FindPrev: *(Self::bind(&library, "FPDFText_FindPrev")?),
            extern_FPDFText_GetSchResultIndex: *(Self::bind(&library, "FPDFText_GetSchResultIndex")?),
            extern_FPDFText_GetSchCount: *(Self::bind(&library, "FPDFText_GetSchCount")?),
            extern_FPDFText_FindClose: *(Self::bind(&library, "FPDFText_FindClose")?),
            extern_FPDFLink_LoadWebLinks: *(Self::bind(&library, "FPDFLink_LoadWebLinks")?),
            extern_FPDFLink_CountWebLinks: *(Self::bind(&library, "FPDFLink_CountWebLinks")?),
            extern_FPDFLink_GetURL: *(Self::bind(&library, "FPDFLink_GetURL")?),
            extern_FPDFLink_CountRects: *(Self::bind(&library, "FPDFLink_CountRects")?),
            extern_FPDFLink_GetRect: *(Self::bind(&library, "FPDFLink_GetRect")?),
            extern_FPDFLink_GetTextRange: *(Self::bind(&library, "FPDFLink_GetTextRange")?),
            extern_FPDFLink_CloseWebLinks: *(Self::bind(&library, "FPDFLink_CloseWebLinks")?),
            extern_FPDFPage_GetDecodedThumbnailData: *(Self::bind(&library, "FPDFPage_GetDecodedThumbnailData")?),
            extern_FPDFPage_GetRawThumbnailData: *(Self::bind(&library, "FPDFPage_GetRawThumbnailData")?),
            extern_FPDFPage_GetThumbnailAsBitmap: *(Self::bind(&library, "FPDFPage_GetThumbnailAsBitmap")?),
            extern_FPDFFormObj_CountObjects: *(Self::bind(&library, "FPDFFormObj_CountObjects")?),
            extern_FPDFFormObj_GetObject: *(Self::bind(&library, "FPDFFormObj_GetObject")?),
            extern_FPDFFormObj_RemoveObject: (*Self::bind(&library, "FPDFFormObj_RemoveObject")?),
            extern_FPDFPageObj_CreateTextObj: *(Self::bind(&library, "FPDFPageObj_CreateTextObj")?),
            extern_FPDFTextObj_GetTextRenderMode: *(Self::bind(&library, "FPDFTextObj_GetTextRenderMode")?),
            extern_FPDFTextObj_SetTextRenderMode: *(Self::bind(&library, "FPDFTextObj_SetTextRenderMode")?),
            extern_FPDFTextObj_GetText: *(Self::bind(&library, "FPDFTextObj_GetText")?),
            extern_FPDFTextObj_GetRenderedBitmap: *(Self::bind(&library, "FPDFTextObj_GetRenderedBitmap")?),
            extern_FPDFTextObj_GetFont: *(Self::bind(&library, "FPDFTextObj_GetFont")?),
            extern_FPDFTextObj_GetFontSize: *(Self::bind(&library, "FPDFTextObj_GetFontSize")?),
            extern_FPDFPageObj_NewTextObj: *(Self::bind(&library, "FPDFPageObj_NewTextObj")?),
            extern_FPDFText_SetText: *(Self::bind(&library, "FPDFText_SetText")?),
            extern_FPDFText_SetCharcodes: *(Self::bind(&library, "FPDFText_SetCharcodes")?),
            extern_FPDFText_LoadFont: *(Self::bind(&library, "FPDFText_LoadFont")?),
            extern_FPDFText_LoadStandardFont: *(Self::bind(&library, "FPDFText_LoadStandardFont")?),
            extern_FPDFText_LoadCidType2Font: *(Self::bind(&library, "FPDFText_LoadCidType2Font")?),
            extern_FPDFFont_Close: *(Self::bind(&library, "FPDFFont_Close")?),
            extern_FPDFPath_MoveTo: *(Self::bind(&library, "FPDFPath_MoveTo")?),
            extern_FPDFPath_LineTo: *(Self::bind(&library, "FPDFPath_LineTo")?),
            extern_FPDFPath_BezierTo: *(Self::bind(&library, "FPDFPath_BezierTo")?),
            extern_FPDFPath_Close: *(Self::bind(&library, "FPDFPath_Close")?),
            extern_FPDFPath_SetDrawMode: *(Self::bind(&library, "FPDFPath_SetDrawMode")?),
            extern_FPDFPath_GetDrawMode: *(Self::bind(&library, "FPDFPath_GetDrawMode")?),
            extern_FPDFPage_InsertObject: *(Self::bind(&library, "FPDFPage_InsertObject")?),
            extern_FPDFPage_InsertObjectAtIndex: *(Self::bind(&library, "FPDFPage_InsertObjectAtIndex")?),
            extern_FPDFPage_RemoveObject: *(Self::bind(&library, "FPDFPage_RemoveObject")?),
            extern_FPDFPage_CountObjects: *(Self::bind(&library, "FPDFPage_CountObjects")?),
            extern_FPDFPage_GetObject: *(Self::bind(&library, "FPDFPage_GetObject")?),
            extern_FPDFPageObj_Destroy: *(Self::bind(&library, "FPDFPageObj_Destroy")?),
            extern_FPDFPageObj_HasTransparency: *(Self::bind(&library, "FPDFPageObj_HasTransparency")?),
            extern_FPDFPageObj_GetType: *(Self::bind(&library, "FPDFPageObj_GetType")?),
            extern_FPDFPageObj_GetIsActive: *(Self::bind(&library, "FPDFPageObj_GetIsActive")?),
            extern_FPDFPageObj_SetIsActive: *(Self::bind(&library, "FPDFPageObj_SetIsActive")?),
            extern_FPDFPageObj_Transform: *(Self::bind(&library, "FPDFPageObj_Transform")?),
            extern_FPDFPageObj_TransformF: *(Self::bind(&library, "FPDFPageObj_TransformF")?),
            extern_FPDFPageObj_GetMatrix: *(Self::bind(&library, "FPDFPageObj_GetMatrix")?),
            extern_FPDFPageObj_SetMatrix: *(Self::bind(&library, "FPDFPageObj_SetMatrix")?),
            extern_FPDFPageObj_NewImageObj: *(Self::bind(&library, "FPDFPageObj_NewImageObj")?),
            extern_FPDFPageObj_GetMarkedContentID: *(Self::bind(&library, "FPDFPageObj_GetMarkedContentID")?),
            extern_FPDFPageObj_CountMarks: *(Self::bind(&library, "FPDFPageObj_CountMarks")?),
            extern_FPDFPageObj_GetMark: *(Self::bind(&library, "FPDFPageObj_GetMark")?),
            extern_FPDFPageObj_AddMark: *(Self::bind(&library, "FPDFPageObj_AddMark")?),
            extern_FPDFPageObj_RemoveMark: *(Self::bind(&library, "FPDFPageObj_RemoveMark")?),
            extern_FPDFPageObjMark_GetName: *(Self::bind(&library, "FPDFPageObjMark_GetName")?),
            extern_FPDFPageObjMark_CountParams: *(Self::bind(&library, "FPDFPageObjMark_CountParams")?),
            extern_FPDFPageObjMark_GetParamKey: *(Self::bind(&library, "FPDFPageObjMark_GetParamKey")?),
            extern_FPDFPageObjMark_GetParamValueType: *(Self::bind(&library, "FPDFPageObjMark_GetParamValueType")?),
            extern_FPDFPageObjMark_GetParamIntValue: *(Self::bind(&library, "FPDFPageObjMark_GetParamIntValue")?),
            extern_FPDFPageObjMark_GetParamFloatValue: *(Self::bind(&library, "FPDFPageObjMark_GetParamFloatValue")?),
            extern_FPDFPageObjMark_GetParamStringValue: *(Self::bind(&library, "FPDFPageObjMark_GetParamStringValue")?),
            extern_FPDFPageObjMark_GetParamBlobValue: *(Self::bind(&library, "FPDFPageObjMark_GetParamBlobValue")?),
            extern_FPDFPageObjMark_SetIntParam: *(Self::bind(&library, "FPDFPageObjMark_SetIntParam")?),
            extern_FPDFPageObjMark_SetFloatParam: *(Self::bind(&library, "FPDFPageObjMark_SetFloatParam")?),
            extern_FPDFPageObjMark_SetStringParam: *(Self::bind(&library, "FPDFPageObjMark_SetStringParam")?),
            extern_FPDFPageObjMark_SetBlobParam: *(Self::bind(&library, "FPDFPageObjMark_SetBlobParam")?),
            extern_FPDFPageObjMark_RemoveParam: *(Self::bind(&library, "FPDFPageObjMark_RemoveParam")?),
            extern_FPDFImageObj_LoadJpegFile: *(Self::bind(&library, "FPDFImageObj_LoadJpegFile")?),
            extern_FPDFImageObj_LoadJpegFileInline: *(Self::bind(&library, "FPDFImageObj_LoadJpegFileInline")?),
            extern_FPDFImageObj_SetMatrix: *(Self::bind(&library, "FPDFImageObj_SetMatrix")?),
            extern_FPDFImageObj_SetBitmap: *(Self::bind(&library, "FPDFImageObj_SetBitmap")?),
            extern_FPDFImageObj_GetBitmap: *(Self::bind(&library, "FPDFImageObj_GetBitmap")?),
            extern_FPDFImageObj_GetRenderedBitmap: *(Self::bind(&library, "FPDFImageObj_GetRenderedBitmap")?),
            extern_FPDFImageObj_GetImageDataDecoded: *(Self::bind(&library, "FPDFImageObj_GetImageDataDecoded")?),
            extern_FPDFImageObj_GetImageDataRaw: *(Self::bind(&library, "FPDFImageObj_GetImageDataRaw")?),
            extern_FPDFImageObj_GetImageFilterCount: *(Self::bind(&library, "FPDFImageObj_GetImageFilterCount")?),
            extern_FPDFImageObj_GetImageFilter: *(Self::bind(&library, "FPDFImageObj_GetImageFilter")?),
            extern_FPDFImageObj_GetImageMetadata: *(Self::bind(&library, "FPDFImageObj_GetImageMetadata")?),
            extern_FPDFImageObj_GetImagePixelSize: *(Self::bind(&library, "FPDFImageObj_GetImagePixelSize")?),
            extern_FPDFImageObj_GetIccProfileDataDecoded: *(Self::bind(
                &library,
                "FPDFImageObj_GetIccProfileDataDecoded",
            )?),
            extern_FPDFPageObj_CreateNewPath: *(Self::bind(&library, "FPDFPageObj_CreateNewPath")?),
            extern_FPDFPageObj_CreateNewRect: *(Self::bind(&library, "FPDFPageObj_CreateNewRect")?),
            extern_FPDFPageObj_GetBounds: *(Self::bind(&library, "FPDFPageObj_GetBounds")?),
            extern_FPDFPageObj_GetRotatedBounds: *(Self::bind(&library, "FPDFPageObj_GetRotatedBounds")?),
            extern_FPDFPageObj_SetBlendMode: *(Self::bind(&library, "FPDFPageObj_SetBlendMode")?),
            extern_FPDFPageObj_SetStrokeColor: *(Self::bind(&library, "FPDFPageObj_SetStrokeColor")?),
            extern_FPDFPageObj_GetStrokeColor: *(Self::bind(&library, "FPDFPageObj_GetStrokeColor")?),
            extern_FPDFPageObj_SetStrokeWidth: *(Self::bind(&library, "FPDFPageObj_SetStrokeWidth")?),
            extern_FPDFPageObj_GetStrokeWidth: *(Self::bind(&library, "FPDFPageObj_GetStrokeWidth")?),
            extern_FPDFPageObj_GetLineJoin: *(Self::bind(&library, "FPDFPageObj_GetLineJoin")?),
            extern_FPDFPageObj_SetLineJoin: *(Self::bind(&library, "FPDFPageObj_SetLineJoin")?),
            extern_FPDFPageObj_GetLineCap: *(Self::bind(&library, "FPDFPageObj_GetLineCap")?),
            extern_FPDFPageObj_SetLineCap: *(Self::bind(&library, "FPDFPageObj_SetLineCap")?),
            extern_FPDFPageObj_SetFillColor: *(Self::bind(&library, "FPDFPageObj_SetFillColor")?),
            extern_FPDFPageObj_GetFillColor: *(Self::bind(&library, "FPDFPageObj_GetFillColor")?),
            extern_FPDFPageObj_GetDashPhase: *(Self::bind(&library, "FPDFPageObj_GetDashPhase")?),
            extern_FPDFPageObj_SetDashPhase: *(Self::bind(&library, "FPDFPageObj_SetDashPhase")?),
            extern_FPDFPageObj_GetDashCount: *(Self::bind(&library, "FPDFPageObj_GetDashCount")?),
            extern_FPDFPageObj_GetDashArray: *(Self::bind(&library, "FPDFPageObj_GetDashArray")?),
            extern_FPDFPageObj_SetDashArray: *(Self::bind(&library, "FPDFPageObj_SetDashArray")?),
            extern_FPDFPath_CountSegments: *(Self::bind(&library, "FPDFPath_CountSegments")?),
            extern_FPDFPath_GetPathSegment: *(Self::bind(&library, "FPDFPath_GetPathSegment")?),
            extern_FPDFPathSegment_GetPoint: *(Self::bind(&library, "FPDFPathSegment_GetPoint")?),
            extern_FPDFPathSegment_GetType: *(Self::bind(&library, "FPDFPathSegment_GetType")?),
            extern_FPDFPathSegment_GetClose: *(Self::bind(&library, "FPDFPathSegment_GetClose")?),
            extern_FPDFFont_GetBaseFontName: *(Self::bind(&library, "FPDFFont_GetBaseFontName")?),
            extern_FPDFFont_GetFamilyName: *(Self::bind(&library, "FPDFFont_GetFamilyName")?),
            extern_FPDFFont_GetFontData: *(Self::bind(&library, "FPDFFont_GetFontData")?),
            extern_FPDFFont_GetIsEmbedded: *(Self::bind(&library, "FPDFFont_GetIsEmbedded")?),
            extern_FPDFFont_GetFlags: *(Self::bind(&library, "FPDFFont_GetFlags")?),
            extern_FPDFFont_GetWeight: *(Self::bind(&library, "FPDFFont_GetWeight")?),
            extern_FPDFFont_GetItalicAngle: *(Self::bind(&library, "FPDFFont_GetItalicAngle")?),
            extern_FPDFFont_GetAscent: *(Self::bind(&library, "FPDFFont_GetAscent")?),
            extern_FPDFFont_GetDescent: *(Self::bind(&library, "FPDFFont_GetDescent")?),
            extern_FPDFFont_GetGlyphWidth: *(Self::bind(&library, "FPDFFont_GetGlyphWidth")?),
            extern_FPDFFont_GetGlyphPath: *(Self::bind(&library, "FPDFFont_GetGlyphPath")?),
            extern_FPDFGlyphPath_CountGlyphSegments: *(Self::bind(&library, "FPDFGlyphPath_CountGlyphSegments")?),
            extern_FPDFGlyphPath_GetGlyphPathSegment: *(Self::bind(&library, "FPDFGlyphPath_GetGlyphPathSegment")?),
            extern_FPDF_VIEWERREF_GetPrintScaling: *(Self::bind(&library, "FPDF_VIEWERREF_GetPrintScaling")?),
            extern_FPDF_VIEWERREF_GetNumCopies: *(Self::bind(&library, "FPDF_VIEWERREF_GetNumCopies")?),
            extern_FPDF_VIEWERREF_GetPrintPageRange: *(Self::bind(&library, "FPDF_VIEWERREF_GetPrintPageRange")?),
            extern_FPDF_VIEWERREF_GetPrintPageRangeCount: *(Self::bind(
                &library,
                "FPDF_VIEWERREF_GetPrintPageRangeCount",
            )?),
            extern_FPDF_VIEWERREF_GetPrintPageRangeElement: *(Self::bind(
                &library,
                "FPDF_VIEWERREF_GetPrintPageRangeElement",
            )?),
            extern_FPDF_VIEWERREF_GetDuplex: *(Self::bind(&library, "FPDF_VIEWERREF_GetDuplex")?),
            extern_FPDF_VIEWERREF_GetName: *(Self::bind(&library, "FPDF_VIEWERREF_GetName")?),
            extern_FPDF_CountNamedDests: *(Self::bind(&library, "FPDF_CountNamedDests")?),
            extern_FPDF_GetNamedDestByName: *(Self::bind(&library, "FPDF_GetNamedDestByName")?),
            extern_FPDF_GetNamedDest: *(Self::bind(&library, "FPDF_GetNamedDest")?),
            extern_FPDFDoc_GetAttachmentCount: *(Self::bind(&library, "FPDFDoc_GetAttachmentCount")?),
            extern_FPDFDoc_AddAttachment: *(Self::bind(&library, "FPDFDoc_AddAttachment")?),
            extern_FPDFDoc_GetAttachment: *(Self::bind(&library, "FPDFDoc_GetAttachment")?),
            extern_FPDFDoc_DeleteAttachment: *(Self::bind(&library, "FPDFDoc_DeleteAttachment")?),
            extern_FPDFAttachment_GetName: *(Self::bind(&library, "FPDFAttachment_GetName")?),
            extern_FPDFAttachment_HasKey: *(Self::bind(&library, "FPDFAttachment_HasKey")?),
            extern_FPDFAttachment_GetValueType: *(Self::bind(&library, "FPDFAttachment_GetValueType")?),
            extern_FPDFAttachment_SetStringValue: *(Self::bind(&library, "FPDFAttachment_SetStringValue")?),
            extern_FPDFAttachment_GetStringValue: *(Self::bind(&library, "FPDFAttachment_GetStringValue")?),
            extern_FPDFAttachment_SetFile: *(Self::bind(&library, "FPDFAttachment_SetFile")?),
            extern_FPDFAttachment_GetFile: *(Self::bind(&library, "FPDFAttachment_GetFile")?),
            extern_FPDFAttachment_GetSubtype: *(Self::bind(&library, "FPDFAttachment_GetSubtype")?),
            extern_FPDFCatalog_IsTagged: *(Self::bind(&library, "FPDFCatalog_IsTagged")?),
            extern_FPDFCatalog_SetLanguage: *(Self::bind(&library, "FPDFCatalog_SetLanguage")?),
            library,
        })
    }
}

impl PdfiumLibraryBindings for DynamicPdfiumBindings {
    #[inline]
    #[allow(non_snake_case)]
    fn FPDF_InitLibraryWithConfig(&self, config: *const FPDF_LIBRARY_CONFIG) {
        unsafe { (self.extern_FPDF_InitLibraryWithConfig)(config) }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FPDF_InitLibrary(&self) {
        unsafe {
            (self.extern_FPDF_InitLibrary)();
        }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FPDF_SetSandBoxPolicy(&self, policy: FPDF_DWORD, enable: FPDF_BOOL) {
        unsafe {
            (self.extern_FPDF_SetSandBoxPolicy)(policy, enable);
        }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FPDF_DestroyLibrary(&self) {
        unsafe {
            (self.extern_FPDF_DestroyLibrary)();
        }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FPDF_GetLastError(&self) -> c_ulong {
        unsafe { (self.extern_FPDF_GetLastError)() }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FPDF_CreateNewDocument(&self) -> FPDF_DOCUMENT {
        unsafe { (self.extern_FPDF_CreateNewDocument)() }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FPDF_LoadDocument(&self, file_path: &str, password: Option<&str>) -> FPDF_DOCUMENT {
        let c_file_path = safe_cstring(file_path);
        let c_password = safe_cstring(password.unwrap_or(""));

        unsafe { (self.extern_FPDF_LoadDocument)(c_file_path.as_ptr(), c_password.as_ptr()) }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FPDF_LoadMemDocument64(&self, bytes: &[u8], password: Option<&str>) -> FPDF_DOCUMENT {
        let c_password = safe_cstring(password.unwrap_or(""));

        unsafe {
            (self.extern_FPDF_LoadMemDocument64)(
                bytes.as_ptr() as *const c_void,
                bytes.len() as c_ulong,
                c_password.as_ptr(),
            )
        }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FPDF_LoadCustomDocument(&self, pFileAccess: *mut FPDF_FILEACCESS, password: Option<&str>) -> FPDF_DOCUMENT {
        let c_password = safe_cstring(password.unwrap_or(""));

        unsafe { (self.extern_FPDF_LoadCustomDocument)(pFileAccess, c_password.as_ptr()) }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FPDF_SaveAsCopy(
        &self,
        document: FPDF_DOCUMENT,
        pFileWrite: *mut FPDF_FILEWRITE,
        flags: FPDF_DWORD,
    ) -> FPDF_BOOL {
        unsafe { (self.extern_FPDF_SaveAsCopy)(document, pFileWrite, flags) }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FPDF_SaveWithVersion(
        &self,
        document: FPDF_DOCUMENT,
        pFileWrite: *mut FPDF_FILEWRITE,
        flags: FPDF_DWORD,
        fileVersion: c_int,
    ) -> FPDF_BOOL {
        unsafe { (self.extern_FPDF_SaveWithVersion)(document, pFileWrite, flags, fileVersion) }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FPDFAvail_Create(&self, file_avail: *mut FX_FILEAVAIL, file: *mut FPDF_FILEACCESS) -> FPDF_AVAIL {
        unsafe { (self.extern_FPDFAvail_Create)(file_avail, file) }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FPDFAvail_Destroy(&self, avail: FPDF_AVAIL) {
        unsafe { (self.extern_FPDFAvail_Destroy)(avail) }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FPDFAvail_IsDocAvail(&self, avail: FPDF_AVAIL, hints: *mut FX_DOWNLOADHINTS) -> c_int {
        unsafe { (self.extern_FPDFAvail_IsDocAvail)(avail, hints) }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FPDFAvail_GetDocument(&self, avail: FPDF_AVAIL, password: Option<&str>) -> FPDF_DOCUMENT {
        let c_password = safe_cstring(password.unwrap_or(""));

        unsafe { (self.extern_FPDFAvail_GetDocument)(avail, c_password.as_ptr()) }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FPDFAvail_GetFirstPageNum(&self, doc: FPDF_DOCUMENT) -> c_int {
        unsafe { (self.extern_FPDFAvail_GetFirstPageNum)(doc) }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FPDFAvail_IsPageAvail(&self, avail: FPDF_AVAIL, page_index: c_int, hints: *mut FX_DOWNLOADHINTS) -> c_int {
        unsafe { (self.extern_FPDFAvail_IsPageAvail)(avail, page_index, hints) }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FPDFAvail_IsFormAvail(&self, avail: FPDF_AVAIL, hints: *mut FX_DOWNLOADHINTS) -> c_int {
        unsafe { (self.extern_FPDFAvail_IsFormAvail)(avail, hints) }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FPDFAvail_IsLinearized(&self, avail: FPDF_AVAIL) -> c_int {
        unsafe { (self.extern_FPDFAvail_IsLinearized)(avail) }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FPDF_CloseDocument(&self, document: FPDF_DOCUMENT) {
        unsafe {
            (self.extern_FPDF_CloseDocument)(document);
        }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FPDF_DeviceToPage(
        &self,
        page: FPDF_PAGE,
        start_x: c_int,
        start_y: c_int,
        size_x: c_int,
        size_y: c_int,
        rotate: c_int,
        device_x: c_int,
        device_y: c_int,
        page_x: *mut c_double,
        page_y: *mut c_double,
    ) -> FPDF_BOOL {
        unsafe {
            (self.extern_FPDF_DeviceToPage)(
                page, start_x, start_y, size_x, size_y, rotate, device_x, device_y, page_x, page_y,
            )
        }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FPDF_PageToDevice(
        &self,
        page: FPDF_PAGE,
        start_x: c_int,
        start_y: c_int,
        size_x: c_int,
        size_y: c_int,
        rotate: c_int,
        page_x: c_double,
        page_y: c_double,
        device_x: *mut c_int,
        device_y: *mut c_int,
    ) -> FPDF_BOOL {
        unsafe {
            (self.extern_FPDF_PageToDevice)(
                page, start_x, start_y, size_x, size_y, rotate, page_x, page_y, device_x, device_y,
            )
        }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FPDF_GetFileVersion(&self, doc: FPDF_DOCUMENT, fileVersion: *mut c_int) -> FPDF_BOOL {
        unsafe { (self.extern_FPDF_GetFileVersion)(doc, fileVersion) }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FPDF_GetFileIdentifier(
        &self,
        document: FPDF_DOCUMENT,
        id_type: FPDF_FILEIDTYPE,
        buffer: *mut c_void,
        buflen: c_ulong,
    ) -> c_ulong {
        unsafe { (self.extern_FPDF_GetFileIdentifier)(document, id_type, buffer, buflen) }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FPDF_GetMetaText(&self, document: FPDF_DOCUMENT, tag: &str, buffer: *mut c_void, buflen: c_ulong) -> c_ulong {
        let c_tag = safe_cstring(tag);

        unsafe { (self.extern_FPDF_GetMetaText)(document, c_tag.as_ptr(), buffer, buflen) }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FPDF_DocumentHasValidCrossReferenceTable(&self, document: FPDF_DOCUMENT) -> FPDF_BOOL {
        unsafe { (self.extern_FPDF_DocumentHasValidCrossReferenceTable)(document) }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FPDF_GetTrailerEnds(&self, document: FPDF_DOCUMENT, buffer: *mut c_uint, length: c_ulong) -> c_ulong {
        unsafe { (self.extern_FPDF_GetTrailerEnds)(document, buffer, length) }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FPDF_GetDocPermissions(&self, document: FPDF_DOCUMENT) -> c_ulong {
        unsafe { (self.extern_FPDF_GetDocPermissions)(document) }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FPDF_GetDocUserPermissions(&self, document: FPDF_DOCUMENT) -> c_ulong {
        unsafe { (self.extern_FPDF_GetDocUserPermissions)(document) }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FPDF_GetSecurityHandlerRevision(&self, document: FPDF_DOCUMENT) -> c_int {
        unsafe { (self.extern_FPDF_GetSecurityHandlerRevision)(document) }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FPDF_GetPageCount(&self, document: FPDF_DOCUMENT) -> c_int {
        unsafe { (self.extern_FPDF_GetPageCount)(document) }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FPDF_LoadPage(&self, document: FPDF_DOCUMENT, page_index: c_int) -> FPDF_PAGE {
        unsafe { (self.extern_FPDF_LoadPage)(document, page_index) }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FPDF_ClosePage(&self, page: FPDF_PAGE) {
        unsafe {
            (self.extern_FPDF_ClosePage)(page);
        }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FPDF_RenderPageBitmapWithColorScheme_Start(
        &self,
        bitmap: FPDF_BITMAP,
        page: FPDF_PAGE,
        start_x: c_int,
        start_y: c_int,
        size_x: c_int,
        size_y: c_int,
        rotate: c_int,
        flags: c_int,
        color_scheme: *const FPDF_COLORSCHEME,
        pause: *mut IFSDK_PAUSE,
    ) -> c_int {
        unsafe {
            (self.extern_FPDF_RenderPageBitmapWithColorScheme_Start)(
                bitmap,
                page,
                start_x,
                start_y,
                size_x,
                size_y,
                rotate,
                flags,
                color_scheme,
                pause,
            )
        }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FPDF_RenderPageBitmap_Start(
        &self,
        bitmap: FPDF_BITMAP,
        page: FPDF_PAGE,
        start_x: c_int,
        start_y: c_int,
        size_x: c_int,
        size_y: c_int,
        rotate: c_int,
        flags: c_int,
        pause: *mut IFSDK_PAUSE,
    ) -> c_int {
        unsafe {
            (self.extern_FPDF_RenderPageBitmap_Start)(
                bitmap, page, start_x, start_y, size_x, size_y, rotate, flags, pause,
            )
        }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FPDF_RenderPage_Continue(&self, page: FPDF_PAGE, pause: *mut IFSDK_PAUSE) -> c_int {
        unsafe { (self.extern_FPDF_RenderPage_Continue)(page, pause) }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FPDF_RenderPage_Close(&self, page: FPDF_PAGE) {
        unsafe { (self.extern_FPDF_RenderPage_Close)(page) }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FPDF_ImportPagesByIndex(
        &self,
        dest_doc: FPDF_DOCUMENT,
        src_doc: FPDF_DOCUMENT,
        page_indices: *const c_int,
        length: c_ulong,
        index: c_int,
    ) -> FPDF_BOOL {
        unsafe { (self.extern_FPDF_ImportPagesByIndex)(dest_doc, src_doc, page_indices, length, index) }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FPDF_ImportPages(
        &self,
        dest_doc: FPDF_DOCUMENT,
        src_doc: FPDF_DOCUMENT,
        pagerange: &str,
        index: c_int,
    ) -> FPDF_BOOL {
        let c_pagerange = safe_cstring(pagerange);

        unsafe { (self.extern_FPDF_ImportPages)(dest_doc, src_doc, c_pagerange.as_ptr(), index) }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FPDF_ImportNPagesToOne(
        &self,
        src_doc: FPDF_DOCUMENT,
        output_width: c_float,
        output_height: c_float,
        num_pages_on_x_axis: size_t,
        num_pages_on_y_axis: size_t,
    ) -> FPDF_DOCUMENT {
        unsafe {
            (self.extern_FPDF_ImportNPagesToOne)(
                src_doc,
                output_width,
                output_height,
                num_pages_on_x_axis,
                num_pages_on_y_axis,
            )
        }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FPDF_NewXObjectFromPage(
        &self,
        dest_doc: FPDF_DOCUMENT,
        src_doc: FPDF_DOCUMENT,
        src_page_index: c_int,
    ) -> FPDF_XOBJECT {
        unsafe { (self.extern_FPDF_NewXObjectFromPage)(dest_doc, src_doc, src_page_index) }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FPDF_CloseXObject(&self, xobject: FPDF_XOBJECT) {
        unsafe { (self.extern_FPDF_CloseXObject)(xobject) };
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FPDF_NewFormObjectFromXObject(&self, xobject: FPDF_XOBJECT) -> FPDF_PAGEOBJECT {
        unsafe { (self.extern_FPDF_NewFormObjectFromXObject)(xobject) }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FPDF_CopyViewerPreferences(&self, dest_doc: FPDF_DOCUMENT, src_doc: FPDF_DOCUMENT) -> FPDF_BOOL {
        unsafe { (self.extern_FPDF_CopyViewerPreferences)(dest_doc, src_doc) }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FPDF_GetPageWidth(&self, page: FPDF_PAGE) -> f64 {
        unsafe { (self.extern_FPDF_GetPageWidth)(page) }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FPDF_GetPageHeight(&self, page: FPDF_PAGE) -> f64 {
        unsafe { (self.extern_FPDF_GetPageHeight)(page) }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FPDF_GetPageWidthF(&self, page: FPDF_PAGE) -> c_float {
        unsafe { (self.extern_FPDF_GetPageWidthF)(page) }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FPDF_GetPageHeightF(&self, page: FPDF_PAGE) -> c_float {
        unsafe { (self.extern_FPDF_GetPageHeightF)(page) }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FPDF_GetPageLabel(
        &self,
        document: FPDF_DOCUMENT,
        page_index: c_int,
        buffer: *mut c_void,
        buflen: c_ulong,
    ) -> c_ulong {
        unsafe { (self.extern_FPDF_GetPageLabel)(document, page_index, buffer, buflen) }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FPDFText_GetCharIndexFromTextIndex(&self, text_page: FPDF_TEXTPAGE, nTextIndex: c_int) -> c_int {
        unsafe { (self.extern_FPDFText_GetCharIndexFromTextIndex)(text_page, nTextIndex) }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FPDFText_GetTextIndexFromCharIndex(&self, text_page: FPDF_TEXTPAGE, nCharIndex: c_int) -> c_int {
        unsafe { (self.extern_FPDFText_GetTextIndexFromCharIndex)(text_page, nCharIndex) }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FPDF_GetSignatureCount(&self, document: FPDF_DOCUMENT) -> c_int {
        unsafe { (self.extern_FPDF_GetSignatureCount)(document) }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FPDF_GetSignatureObject(&self, document: FPDF_DOCUMENT, index: c_int) -> FPDF_SIGNATURE {
        unsafe { (self.extern_FPDF_GetSignatureObject)(document, index) }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FPDFSignatureObj_GetContents(&self, signature: FPDF_SIGNATURE, buffer: *mut c_void, length: c_ulong) -> c_ulong {
        unsafe { (self.extern_FPDFSignatureObj_GetContents)(signature, buffer, length) }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FPDFSignatureObj_GetByteRange(&self, signature: FPDF_SIGNATURE, buffer: *mut c_int, length: c_ulong) -> c_ulong {
        unsafe { (self.extern_FPDFSignatureObj_GetByteRange)(signature, buffer, length) }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FPDFSignatureObj_GetSubFilter(
        &self,
        signature: FPDF_SIGNATURE,
        buffer: *mut c_char,
        length: c_ulong,
    ) -> c_ulong {
        unsafe { (self.extern_FPDFSignatureObj_GetSubFilter)(signature, buffer, length) }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FPDFSignatureObj_GetReason(&self, signature: FPDF_SIGNATURE, buffer: *mut c_void, length: c_ulong) -> c_ulong {
        unsafe { (self.extern_FPDFSignatureObj_GetReason)(signature, buffer, length) }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FPDFSignatureObj_GetTime(&self, signature: FPDF_SIGNATURE, buffer: *mut c_char, length: c_ulong) -> c_ulong {
        unsafe { (self.extern_FPDFSignatureObj_GetTime)(signature, buffer, length) }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FPDFSignatureObj_GetDocMDPPermission(&self, signature: FPDF_SIGNATURE) -> c_uint {
        unsafe { (self.extern_FPDFSignatureObj_GetDocMDPPermission)(signature) }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FPDF_StructTree_GetForPage(&self, page: FPDF_PAGE) -> FPDF_STRUCTTREE {
        unsafe { (self.extern_FPDF_StructTree_GetForPage)(page) }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FPDF_StructTree_Close(&self, struct_tree: FPDF_STRUCTTREE) {
        unsafe { (self.extern_FPDF_StructTree_Close)(struct_tree) }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FPDF_StructTree_CountChildren(&self, struct_tree: FPDF_STRUCTTREE) -> c_int {
        unsafe { (self.extern_FPDF_StructTree_CountChildren)(struct_tree) }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FPDF_StructTree_GetChildAtIndex(&self, struct_tree: FPDF_STRUCTTREE, index: c_int) -> FPDF_STRUCTELEMENT {
        unsafe { (self.extern_FPDF_StructTree_GetChildAtIndex)(struct_tree, index) }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FPDF_StructElement_GetAltText(
        &self,
        struct_element: FPDF_STRUCTELEMENT,
        buffer: *mut c_void,
        buflen: c_ulong,
    ) -> c_ulong {
        unsafe { (self.extern_FPDF_StructElement_GetAltText)(struct_element, buffer, buflen) }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FPDF_StructElement_GetActualText(
        &self,
        struct_element: FPDF_STRUCTELEMENT,
        buffer: *mut c_void,
        buflen: c_ulong,
    ) -> c_ulong {
        unsafe { (self.extern_FPDF_StructElement_GetActualText)(struct_element, buffer, buflen) }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FPDF_StructElement_GetID(
        &self,
        struct_element: FPDF_STRUCTELEMENT,
        buffer: *mut c_void,
        buflen: c_ulong,
    ) -> c_ulong {
        unsafe { (self.extern_FPDF_StructElement_GetID)(struct_element, buffer, buflen) }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FPDF_StructElement_GetLang(
        &self,
        struct_element: FPDF_STRUCTELEMENT,
        buffer: *mut c_void,
        buflen: c_ulong,
    ) -> c_ulong {
        unsafe { (self.extern_FPDF_StructElement_GetLang)(struct_element, buffer, buflen) }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FPDF_StructElement_GetStringAttribute(
        &self,
        struct_element: FPDF_STRUCTELEMENT,
        attr_name: &str,
        buffer: *mut c_void,
        buflen: c_ulong,
    ) -> c_ulong {
        let c_attr_name = safe_cstring(attr_name);

        unsafe {
            (self.extern_FPDF_StructElement_GetStringAttribute)(struct_element, c_attr_name.as_ptr(), buffer, buflen)
        }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FPDF_StructElement_GetMarkedContentID(&self, struct_element: FPDF_STRUCTELEMENT) -> c_int {
        unsafe { (self.extern_FPDF_StructElement_GetMarkedContentID)(struct_element) }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FPDF_StructElement_GetType(
        &self,
        struct_element: FPDF_STRUCTELEMENT,
        buffer: *mut c_void,
        buflen: c_ulong,
    ) -> c_ulong {
        unsafe { (self.extern_FPDF_StructElement_GetType)(struct_element, buffer, buflen) }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FPDF_StructElement_GetObjType(
        &self,
        struct_element: FPDF_STRUCTELEMENT,
        buffer: *mut c_void,
        buflen: c_ulong,
    ) -> c_ulong {
        unsafe { (self.extern_FPDF_StructElement_GetObjType)(struct_element, buffer, buflen) }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FPDF_StructElement_GetTitle(
        &self,
        struct_element: FPDF_STRUCTELEMENT,
        buffer: *mut c_void,
        buflen: c_ulong,
    ) -> c_ulong {
        unsafe { (self.extern_FPDF_StructElement_GetTitle)(struct_element, buffer, buflen) }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FPDF_StructElement_CountChildren(&self, struct_element: FPDF_STRUCTELEMENT) -> c_int {
        unsafe { (self.extern_FPDF_StructElement_CountChildren)(struct_element) }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FPDF_StructElement_GetChildAtIndex(
        &self,
        struct_element: FPDF_STRUCTELEMENT,
        index: c_int,
    ) -> FPDF_STRUCTELEMENT {
        unsafe { (self.extern_FPDF_StructElement_GetChildAtIndex)(struct_element, index) }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FPDF_StructElement_GetChildMarkedContentID(&self, struct_element: FPDF_STRUCTELEMENT, index: c_int) -> c_int {
        unsafe { (self.extern_FPDF_StructElement_GetChildMarkedContentID)(struct_element, index) }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FPDF_StructElement_GetParent(&self, struct_element: FPDF_STRUCTELEMENT) -> FPDF_STRUCTELEMENT {
        unsafe { (self.extern_FPDF_StructElement_GetParent)(struct_element) }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FPDF_StructElement_GetAttributeCount(&self, struct_element: FPDF_STRUCTELEMENT) -> c_int {
        unsafe { (self.extern_FPDF_StructElement_GetAttributeCount)(struct_element) }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FPDF_StructElement_GetAttributeAtIndex(
        &self,
        struct_element: FPDF_STRUCTELEMENT,
        index: c_int,
    ) -> FPDF_STRUCTELEMENT_ATTR {
        unsafe { (self.extern_FPDF_StructElement_GetAttributeAtIndex)(struct_element, index) }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FPDF_StructElement_Attr_GetCount(&self, struct_attribute: FPDF_STRUCTELEMENT_ATTR) -> c_int {
        unsafe { (self.extern_FPDF_StructElement_Attr_GetCount)(struct_attribute) }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FPDF_StructElement_Attr_GetName(
        &self,
        struct_attribute: FPDF_STRUCTELEMENT_ATTR,
        index: c_int,
        buffer: *mut c_void,
        buflen: c_ulong,
        out_buflen: *mut c_ulong,
    ) -> FPDF_BOOL {
        unsafe { (self.extern_FPDF_StructElement_Attr_GetName)(struct_attribute, index, buffer, buflen, out_buflen) }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FPDF_StructElement_Attr_GetValue(
        &self,
        struct_attribute: FPDF_STRUCTELEMENT_ATTR,
        name: &str,
    ) -> FPDF_STRUCTELEMENT_ATTR_VALUE {
        let c_name = safe_cstring(name);

        unsafe { (self.extern_FPDF_StructElement_Attr_GetValue)(struct_attribute, c_name.as_ptr()) }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FPDF_StructElement_Attr_GetType(&self, value: FPDF_STRUCTELEMENT_ATTR_VALUE) -> FPDF_OBJECT_TYPE {
        unsafe { (self.extern_FPDF_StructElement_Attr_GetType)(value) }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FPDF_StructElement_Attr_GetBooleanValue(
        &self,
        value: FPDF_STRUCTELEMENT_ATTR_VALUE,
        out_value: *mut FPDF_BOOL,
    ) -> FPDF_BOOL {
        unsafe { (self.extern_FPDF_StructElement_Attr_GetBooleanValue)(value, out_value) }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FPDF_StructElement_Attr_GetNumberValue(
        &self,
        value: FPDF_STRUCTELEMENT_ATTR_VALUE,
        out_value: *mut f32,
    ) -> FPDF_BOOL {
        unsafe { (self.extern_FPDF_StructElement_Attr_GetNumberValue)(value, out_value) }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FPDF_StructElement_Attr_GetStringValue(
        &self,
        value: FPDF_STRUCTELEMENT_ATTR_VALUE,
        buffer: *mut c_void,
        buflen: c_ulong,
        out_buflen: *mut c_ulong,
    ) -> FPDF_BOOL {
        unsafe { (self.extern_FPDF_StructElement_Attr_GetStringValue)(value, buffer, buflen, out_buflen) }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FPDF_StructElement_Attr_GetBlobValue(
        &self,
        value: FPDF_STRUCTELEMENT_ATTR_VALUE,
        buffer: *mut c_void,
        buflen: c_ulong,
        out_buflen: *mut c_ulong,
    ) -> FPDF_BOOL {
        unsafe { (self.extern_FPDF_StructElement_Attr_GetBlobValue)(value, buffer, buflen, out_buflen) }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FPDF_StructElement_Attr_CountChildren(&self, value: FPDF_STRUCTELEMENT_ATTR_VALUE) -> c_int {
        unsafe { (self.extern_FPDF_StructElement_Attr_CountChildren)(value) }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FPDF_StructElement_Attr_GetChildAtIndex(
        &self,
        value: FPDF_STRUCTELEMENT_ATTR_VALUE,
        index: c_int,
    ) -> FPDF_STRUCTELEMENT_ATTR_VALUE {
        unsafe { (self.extern_FPDF_StructElement_Attr_GetChildAtIndex)(value, index) }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FPDF_StructElement_GetMarkedContentIdCount(&self, struct_element: FPDF_STRUCTELEMENT) -> c_int {
        unsafe { (self.extern_FPDF_StructElement_GetMarkedContentIdCount)(struct_element) }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FPDF_StructElement_GetMarkedContentIdAtIndex(&self, struct_element: FPDF_STRUCTELEMENT, index: c_int) -> c_int {
        unsafe { (self.extern_FPDF_StructElement_GetMarkedContentIdAtIndex)(struct_element, index) }
    }

    #[allow(non_snake_case)]
    fn FPDFPage_New(&self, document: FPDF_DOCUMENT, page_index: c_int, width: c_double, height: c_double) -> FPDF_PAGE {
        unsafe { (self.extern_FPDFPage_New)(document, page_index, width, height) }
    }

    #[allow(non_snake_case)]
    fn FPDFPage_Delete(&self, document: FPDF_DOCUMENT, page_index: c_int) {
        unsafe { (self.extern_FPDFPage_Delete)(document, page_index) }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FPDF_MovePages(
        &self,
        document: FPDF_DOCUMENT,
        page_indices: *const c_int,
        page_indices_len: c_ulong,
        dest_page_index: c_int,
    ) -> FPDF_BOOL {
        unsafe { (self.extern_FPDF_MovePages)(document, page_indices, page_indices_len, dest_page_index) }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FPDFPage_GetRotation(&self, page: FPDF_PAGE) -> c_int {
        unsafe { (self.extern_FPDFPage_GetRotation)(page) }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FPDFPage_SetRotation(&self, page: FPDF_PAGE, rotate: c_int) {
        unsafe { (self.extern_FPDFPage_SetRotation)(page, rotate) }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FPDF_GetPageBoundingBox(&self, page: FPDF_PAGE, rect: *mut FS_RECTF) -> FPDF_BOOL {
        unsafe { (self.extern_FPDF_GetPageBoundingBox)(page, rect) }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FPDF_GetPageSizeByIndexF(&self, document: FPDF_DOCUMENT, page_index: c_int, size: *mut FS_SIZEF) -> FPDF_BOOL {
        unsafe { (self.extern_FPDF_GetPageSizeByIndexF)(document, page_index, size) }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FPDF_GetPageSizeByIndex(
        &self,
        document: FPDF_DOCUMENT,
        page_index: c_int,
        width: *mut f64,
        height: *mut f64,
    ) -> c_int {
        unsafe { (self.extern_FPDF_GetPageSizeByIndex)(document, page_index, width, height) }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FPDFPage_GetMediaBox(
        &self,
        page: FPDF_PAGE,
        left: *mut c_float,
        bottom: *mut c_float,
        right: *mut c_float,
        top: *mut c_float,
    ) -> FPDF_BOOL {
        unsafe { (self.extern_FPDFPage_GetMediaBox)(page, left, bottom, right, top) }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FPDFPage_GetCropBox(
        &self,
        page: FPDF_PAGE,
        left: *mut c_float,
        bottom: *mut c_float,
        right: *mut c_float,
        top: *mut c_float,
    ) -> FPDF_BOOL {
        unsafe { (self.extern_FPDFPage_GetCropBox)(page, left, bottom, right, top) }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FPDFPage_GetBleedBox(
        &self,
        page: FPDF_PAGE,
        left: *mut c_float,
        bottom: *mut c_float,
        right: *mut c_float,
        top: *mut c_float,
    ) -> FPDF_BOOL {
        unsafe { (self.extern_FPDFPage_GetBleedBox)(page, left, bottom, right, top) }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FPDFPage_GetTrimBox(
        &self,
        page: FPDF_PAGE,
        left: *mut c_float,
        bottom: *mut c_float,
        right: *mut c_float,
        top: *mut c_float,
    ) -> FPDF_BOOL {
        unsafe { (self.extern_FPDFPage_GetTrimBox)(page, left, bottom, right, top) }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FPDFPage_GetArtBox(
        &self,
        page: FPDF_PAGE,
        left: *mut c_float,
        bottom: *mut c_float,
        right: *mut c_float,
        top: *mut c_float,
    ) -> FPDF_BOOL {
        unsafe { (self.extern_FPDFPage_GetArtBox)(page, left, bottom, right, top) }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FPDFPage_SetMediaBox(&self, page: FPDF_PAGE, left: c_float, bottom: c_float, right: c_float, top: c_float) {
        unsafe { (self.extern_FPDFPage_SetMediaBox)(page, left, bottom, right, top) }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FPDFPage_SetCropBox(&self, page: FPDF_PAGE, left: c_float, bottom: c_float, right: c_float, top: c_float) {
        unsafe { (self.extern_FPDFPage_SetCropBox)(page, left, bottom, right, top) }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FPDFPage_SetBleedBox(&self, page: FPDF_PAGE, left: c_float, bottom: c_float, right: c_float, top: c_float) {
        unsafe { (self.extern_FPDFPage_SetBleedBox)(page, left, bottom, right, top) }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FPDFPage_SetTrimBox(&self, page: FPDF_PAGE, left: c_float, bottom: c_float, right: c_float, top: c_float) {
        unsafe { (self.extern_FPDFPage_SetTrimBox)(page, left, bottom, right, top) }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FPDFPage_SetArtBox(&self, page: FPDF_PAGE, left: c_float, bottom: c_float, right: c_float, top: c_float) {
        unsafe { (self.extern_FPDFPage_SetArtBox)(page, left, bottom, right, top) }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FPDFPage_TransFormWithClip(
        &self,
        page: FPDF_PAGE,
        matrix: *const FS_MATRIX,
        clipRect: *const FS_RECTF,
    ) -> FPDF_BOOL {
        unsafe { (self.extern_FPDFPage_TransFormWithClip)(page, matrix, clipRect) }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FPDFPageObj_TransformClipPath(
        &self,
        page_object: FPDF_PAGEOBJECT,
        a: f64,
        b: f64,
        c: f64,
        d: f64,
        e: f64,
        f: f64,
    ) {
        unsafe { (self.extern_FPDFPageObj_TransformClipPath)(page_object, a, b, c, d, e, f) }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FPDFPageObj_GetClipPath(&self, page_object: FPDF_PAGEOBJECT) -> FPDF_CLIPPATH {
        unsafe { (self.extern_FPDFPageObj_GetClipPath)(page_object) }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FPDFClipPath_CountPaths(&self, clip_path: FPDF_CLIPPATH) -> c_int {
        unsafe { (self.extern_FPDFClipPath_CountPaths)(clip_path) }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FPDFClipPath_CountPathSegments(&self, clip_path: FPDF_CLIPPATH, path_index: c_int) -> c_int {
        unsafe { (self.extern_FPDFClipPath_CountPathSegments)(clip_path, path_index) }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FPDFClipPath_GetPathSegment(
        &self,
        clip_path: FPDF_CLIPPATH,
        path_index: c_int,
        segment_index: c_int,
    ) -> FPDF_PATHSEGMENT {
        unsafe { (self.extern_FPDFClipPath_GetPathSegment)(clip_path, path_index, segment_index) }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FPDF_CreateClipPath(&self, left: f32, bottom: f32, right: f32, top: f32) -> FPDF_CLIPPATH {
        unsafe { (self.extern_FPDF_CreateClipPath)(left, bottom, right, top) }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FPDF_DestroyClipPath(&self, clipPath: FPDF_CLIPPATH) {
        unsafe { (self.extern_FPDF_DestroyClipPath)(clipPath) }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FPDFPage_InsertClipPath(&self, page: FPDF_PAGE, clipPath: FPDF_CLIPPATH) {
        unsafe { (self.extern_FPDFPage_InsertClipPath)(page, clipPath) }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FPDFPage_HasTransparency(&self, page: FPDF_PAGE) -> FPDF_BOOL {
        unsafe { (self.extern_FPDFPage_HasTransparency)(page) }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FPDFPage_GenerateContent(&self, page: FPDF_PAGE) -> FPDF_BOOL {
        unsafe { (self.extern_FPDFPage_GenerateContent)(page) }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FPDFPage_TransformAnnots(&self, page: FPDF_PAGE, a: f64, b: f64, c: f64, d: f64, e: f64, f: f64) {
        unsafe { (self.extern_FPDFPage_TransformAnnots)(page, a, b, c, d, e, f) }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FPDFBitmap_Create(&self, width: c_int, height: c_int, alpha: c_int) -> FPDF_BITMAP {
        unsafe { (self.extern_FPDFBitmap_Create)(width, height, alpha) }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FPDFBitmap_CreateEx(
        &self,
        width: c_int,
        height: c_int,
        format: c_int,
        first_scan: *mut c_void,
        stride: c_int,
    ) -> FPDF_BITMAP {
        unsafe { (self.extern_FPDFBitmap_CreateEx)(width, height, format, first_scan, stride) }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FPDFBitmap_Destroy(&self, bitmap: FPDF_BITMAP) {
        unsafe { (self.extern_FPDFBitmap_Destroy)(bitmap) }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FPDFBitmap_GetFormat(&self, bitmap: FPDF_BITMAP) -> c_int {
        unsafe { (self.extern_FPDFBitmap_GetFormat)(bitmap) }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FPDFBitmap_FillRect(
        &self,
        bitmap: FPDF_BITMAP,
        left: c_int,
        top: c_int,
        width: c_int,
        height: c_int,
        color: FPDF_DWORD,
    ) -> FPDF_BOOL {
        unsafe { (self.extern_FPDFBitmap_FillRect)(bitmap, left, top, width, height, color) }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FPDFBitmap_GetBuffer(&self, bitmap: FPDF_BITMAP) -> *mut c_void {
        unsafe { (self.extern_FPDFBitmap_GetBuffer)(bitmap) }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FPDFBitmap_GetWidth(&self, bitmap: FPDF_BITMAP) -> c_int {
        unsafe { (self.extern_FPDFBitmap_GetWidth)(bitmap) }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FPDFBitmap_GetHeight(&self, bitmap: FPDF_BITMAP) -> c_int {
        unsafe { (self.extern_FPDFBitmap_GetHeight)(bitmap) }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FPDFBitmap_GetStride(&self, bitmap: FPDF_BITMAP) -> c_int {
        unsafe { (self.extern_FPDFBitmap_GetStride)(bitmap) }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FPDF_RenderPageBitmap(
        &self,
        bitmap: FPDF_BITMAP,
        page: FPDF_PAGE,
        start_x: c_int,
        start_y: c_int,
        size_x: c_int,
        size_y: c_int,
        rotate: c_int,
        flags: c_int,
    ) {
        unsafe {
            (self.extern_FPDF_RenderPageBitmap)(bitmap, page, start_x, start_y, size_x, size_y, rotate, flags);
        }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FPDF_RenderPageBitmapWithMatrix(
        &self,
        bitmap: FPDF_BITMAP,
        page: FPDF_PAGE,
        matrix: *const FS_MATRIX,
        clipping: *const FS_RECTF,
        flags: c_int,
    ) {
        unsafe {
            (self.extern_FPDF_RenderPageBitmapWithMatrix)(bitmap, page, matrix, clipping, flags);
        }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FPDFAnnot_IsSupportedSubtype(&self, subtype: FPDF_ANNOTATION_SUBTYPE) -> FPDF_BOOL {
        unsafe { (self.extern_FPDFAnnot_IsSupportedSubtype)(subtype) }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FPDFPage_CreateAnnot(&self, page: FPDF_PAGE, subtype: FPDF_ANNOTATION_SUBTYPE) -> FPDF_ANNOTATION {
        unsafe { (self.extern_FPDFPage_CreateAnnot)(page, subtype) }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FPDFPage_GetAnnotCount(&self, page: FPDF_PAGE) -> c_int {
        unsafe { (self.extern_FPDFPage_GetAnnotCount)(page) }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FPDFPage_GetAnnot(&self, page: FPDF_PAGE, index: c_int) -> FPDF_ANNOTATION {
        unsafe { (self.extern_FPDFPage_GetAnnot)(page, index) }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FPDFPage_GetAnnotIndex(&self, page: FPDF_PAGE, annot: FPDF_ANNOTATION) -> c_int {
        unsafe { (self.extern_FPDFPage_GetAnnotIndex)(page, annot) }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FPDFPage_CloseAnnot(&self, annot: FPDF_ANNOTATION) {
        unsafe { (self.extern_FPDFPage_CloseAnnot)(annot) }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FPDFPage_RemoveAnnot(&self, page: FPDF_PAGE, index: c_int) -> FPDF_BOOL {
        unsafe { (self.extern_FPDFPage_RemoveAnnot)(page, index) }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FPDFAnnot_GetSubtype(&self, annot: FPDF_ANNOTATION) -> FPDF_ANNOTATION_SUBTYPE {
        unsafe { (self.extern_FPDFAnnot_GetSubtype)(annot) }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FPDFAnnot_IsObjectSupportedSubtype(&self, subtype: FPDF_ANNOTATION_SUBTYPE) -> FPDF_BOOL {
        unsafe { (self.extern_FPDFAnnot_IsObjectSupportedSubtype)(subtype) }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FPDFAnnot_UpdateObject(&self, annot: FPDF_ANNOTATION, obj: FPDF_PAGEOBJECT) -> FPDF_BOOL {
        unsafe { (self.extern_FPDFAnnot_UpdateObject)(annot, obj) }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FPDFAnnot_AddInkStroke(&self, annot: FPDF_ANNOTATION, points: *const FS_POINTF, point_count: size_t) -> c_int {
        unsafe { (self.extern_FPDFAnnot_AddInkStroke)(annot, points, point_count) }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FPDFAnnot_RemoveInkList(&self, annot: FPDF_ANNOTATION) -> FPDF_BOOL {
        unsafe { (self.extern_FPDFAnnot_RemoveInkList)(annot) }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FPDFAnnot_AppendObject(&self, annot: FPDF_ANNOTATION, obj: FPDF_PAGEOBJECT) -> FPDF_BOOL {
        unsafe { (self.extern_FPDFAnnot_AppendObject)(annot, obj) }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FPDFAnnot_GetObjectCount(&self, annot: FPDF_ANNOTATION) -> c_int {
        unsafe { (self.extern_FPDFAnnot_GetObjectCount)(annot) }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FPDFAnnot_GetObject(&self, annot: FPDF_ANNOTATION, index: c_int) -> FPDF_PAGEOBJECT {
        unsafe { (self.extern_FPDFAnnot_GetObject)(annot, index) }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FPDFAnnot_RemoveObject(&self, annot: FPDF_ANNOTATION, index: c_int) -> FPDF_BOOL {
        unsafe { (self.extern_FPDFAnnot_RemoveObject)(annot, index) }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FPDFAnnot_SetColor(
        &self,
        annot: FPDF_ANNOTATION,
        color_type: FPDFANNOT_COLORTYPE,
        R: c_uint,
        G: c_uint,
        B: c_uint,
        A: c_uint,
    ) -> FPDF_BOOL {
        unsafe { (self.extern_FPDFAnnot_SetColor)(annot, color_type, R, G, B, A) }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FPDFAnnot_GetColor(
        &self,
        annot: FPDF_ANNOTATION,
        color_type: FPDFANNOT_COLORTYPE,
        R: *mut c_uint,
        G: *mut c_uint,
        B: *mut c_uint,
        A: *mut c_uint,
    ) -> FPDF_BOOL {
        unsafe { (self.extern_FPDFAnnot_GetColor)(annot, color_type, R, G, B, A) }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FPDFAnnot_HasAttachmentPoints(&self, annot: FPDF_ANNOTATION) -> FPDF_BOOL {
        unsafe { (self.extern_FPDFAnnot_HasAttachmentPoints)(annot) }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FPDFAnnot_SetAttachmentPoints(
        &self,
        annot: FPDF_ANNOTATION,
        quad_index: size_t,
        quad_points: *const FS_QUADPOINTSF,
    ) -> FPDF_BOOL {
        unsafe { (self.extern_FPDFAnnot_SetAttachmentPoints)(annot, quad_index, quad_points) }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FPDFAnnot_AppendAttachmentPoints(
        &self,
        annot: FPDF_ANNOTATION,
        quad_points: *const FS_QUADPOINTSF,
    ) -> FPDF_BOOL {
        unsafe { (self.extern_FPDFAnnot_AppendAttachmentPoints)(annot, quad_points) }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FPDFAnnot_CountAttachmentPoints(&self, annot: FPDF_ANNOTATION) -> size_t {
        unsafe { (self.extern_FPDFAnnot_CountAttachmentPoints)(annot) }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FPDFAnnot_GetAttachmentPoints(
        &self,
        annot: FPDF_ANNOTATION,
        quad_index: size_t,
        quad_points: *mut FS_QUADPOINTSF,
    ) -> FPDF_BOOL {
        unsafe { (self.extern_FPDFAnnot_GetAttachmentPoints)(annot, quad_index, quad_points) }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FPDFAnnot_SetRect(&self, annot: FPDF_ANNOTATION, rect: *const FS_RECTF) -> FPDF_BOOL {
        unsafe { (self.extern_FPDFAnnot_SetRect)(annot, rect) }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FPDFAnnot_GetRect(&self, annot: FPDF_ANNOTATION, rect: *mut FS_RECTF) -> FPDF_BOOL {
        unsafe { (self.extern_FPDFAnnot_GetRect)(annot, rect) }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FPDFAnnot_GetVertices(&self, annot: FPDF_ANNOTATION, buffer: *mut FS_POINTF, length: c_ulong) -> c_ulong {
        unsafe { (self.extern_FPDFAnnot_GetVertices)(annot, buffer, length) }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FPDFAnnot_GetInkListCount(&self, annot: FPDF_ANNOTATION) -> c_ulong {
        unsafe { (self.extern_FPDFAnnot_GetInkListCount)(annot) }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FPDFAnnot_GetInkListPath(
        &self,
        annot: FPDF_ANNOTATION,
        path_index: c_ulong,
        buffer: *mut FS_POINTF,
        length: c_ulong,
    ) -> c_ulong {
        unsafe { (self.extern_FPDFAnnot_GetInkListPath)(annot, path_index, buffer, length) }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FPDFAnnot_GetLine(&self, annot: FPDF_ANNOTATION, start: *mut FS_POINTF, end: *mut FS_POINTF) -> FPDF_BOOL {
        unsafe { (self.extern_FPDFAnnot_GetLine)(annot, start, end) }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FPDFAnnot_SetBorder(
        &self,
        annot: FPDF_ANNOTATION,
        horizontal_radius: f32,
        vertical_radius: f32,
        border_width: f32,
    ) -> FPDF_BOOL {
        unsafe { (self.extern_FPDFAnnot_SetBorder)(annot, horizontal_radius, vertical_radius, border_width) }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FPDFAnnot_GetBorder(
        &self,
        annot: FPDF_ANNOTATION,
        horizontal_radius: *mut f32,
        vertical_radius: *mut f32,
        border_width: *mut f32,
    ) -> FPDF_BOOL {
        unsafe { (self.extern_FPDFAnnot_GetBorder)(annot, horizontal_radius, vertical_radius, border_width) }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FPDFAnnot_GetFormAdditionalActionJavaScript(
        &self,
        form: FPDF_FORMHANDLE,
        annot: FPDF_ANNOTATION,
        event: c_int,
        buffer: *mut FPDF_WCHAR,
        buflen: c_ulong,
    ) -> c_ulong {
        unsafe { (self.extern_FPDFAnnot_GetFormAdditionalActionJavaScript)(form, annot, event, buffer, buflen) }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FPDFAnnot_GetFormFieldAlternateName(
        &self,
        form: FPDF_FORMHANDLE,
        annot: FPDF_ANNOTATION,
        buffer: *mut FPDF_WCHAR,
        buflen: c_ulong,
    ) -> c_ulong {
        unsafe { (self.extern_FPDFAnnot_GetFormFieldAlternateName)(form, annot, buffer, buflen) }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FPDFAnnot_HasKey(&self, annot: FPDF_ANNOTATION, key: &str) -> FPDF_BOOL {
        let c_key = safe_cstring(key);

        unsafe { (self.extern_FPDFAnnot_HasKey)(annot, c_key.as_ptr()) }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FPDFAnnot_GetValueType(&self, annot: FPDF_ANNOTATION, key: &str) -> FPDF_OBJECT_TYPE {
        let c_key = safe_cstring(key);

        unsafe { (self.extern_FPDFAnnot_GetValueType)(annot, c_key.as_ptr()) }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FPDFAnnot_SetStringValue(&self, annot: FPDF_ANNOTATION, key: &str, value: FPDF_WIDESTRING) -> FPDF_BOOL {
        let c_key = safe_cstring(key);

        unsafe { (self.extern_FPDFAnnot_SetStringValue)(annot, c_key.as_ptr(), value) }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FPDFAnnot_GetStringValue(
        &self,
        annot: FPDF_ANNOTATION,
        key: &str,
        buffer: *mut FPDF_WCHAR,
        buflen: c_ulong,
    ) -> c_ulong {
        let c_key = safe_cstring(key);

        unsafe { (self.extern_FPDFAnnot_GetStringValue)(annot, c_key.as_ptr(), buffer, buflen) }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FPDFAnnot_GetNumberValue(&self, annot: FPDF_ANNOTATION, key: &str, value: *mut f32) -> FPDF_BOOL {
        let c_key = safe_cstring(key);

        unsafe { (self.extern_FPDFAnnot_GetNumberValue)(annot, c_key.as_ptr(), value) }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FPDFAnnot_SetAP(
        &self,
        annot: FPDF_ANNOTATION,
        appearanceMode: FPDF_ANNOT_APPEARANCEMODE,
        value: FPDF_WIDESTRING,
    ) -> FPDF_BOOL {
        unsafe { (self.extern_FPDFAnnot_SetAP)(annot, appearanceMode, value) }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FPDFAnnot_GetAP(
        &self,
        annot: FPDF_ANNOTATION,
        appearanceMode: FPDF_ANNOT_APPEARANCEMODE,
        buffer: *mut FPDF_WCHAR,
        buflen: c_ulong,
    ) -> c_ulong {
        unsafe { (self.extern_FPDFAnnot_GetAP)(annot, appearanceMode, buffer, buflen) }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FPDFAnnot_GetLinkedAnnot(&self, annot: FPDF_ANNOTATION, key: &str) -> FPDF_ANNOTATION {
        let c_key = safe_cstring(key);

        unsafe { (self.extern_FPDFAnnot_GetLinkedAnnot)(annot, c_key.as_ptr()) }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FPDFAnnot_GetFlags(&self, annot: FPDF_ANNOTATION) -> c_int {
        unsafe { (self.extern_FPDFAnnot_GetFlags)(annot) }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FPDFAnnot_SetFlags(&self, annot: FPDF_ANNOTATION, flags: c_int) -> FPDF_BOOL {
        unsafe { (self.extern_FPDFAnnot_SetFlags)(annot, flags) }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FPDFAnnot_GetFormFieldFlags(&self, form: FPDF_FORMHANDLE, annot: FPDF_ANNOTATION) -> c_int {
        unsafe { (self.extern_FPDFAnnot_GetFormFieldFlags)(form, annot) }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FPDFAnnot_SetFormFieldFlags(&self, form: FPDF_FORMHANDLE, annot: FPDF_ANNOTATION, flags: c_int) -> FPDF_BOOL {
        unsafe { (self.extern_FPDFAnnot_SetFormFieldFlags)(form, annot, flags) }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FPDFAnnot_GetFormFieldAtPoint(
        &self,
        form: FPDF_FORMHANDLE,
        page: FPDF_PAGE,
        point: *const FS_POINTF,
    ) -> FPDF_ANNOTATION {
        unsafe { (self.extern_FPDFAnnot_GetFormFieldAtPoint)(form, page, point) }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FPDFAnnot_GetFormFieldName(
        &self,
        form: FPDF_FORMHANDLE,
        annot: FPDF_ANNOTATION,
        buffer: *mut FPDF_WCHAR,
        buflen: c_ulong,
    ) -> c_ulong {
        unsafe { (self.extern_FPDFAnnot_GetFormFieldName)(form, annot, buffer, buflen) }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FPDFAnnot_GetFormFieldType(&self, form: FPDF_FORMHANDLE, annot: FPDF_ANNOTATION) -> c_int {
        unsafe { (self.extern_FPDFAnnot_GetFormFieldType)(form, annot) }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FPDFAnnot_GetFormFieldValue(
        &self,
        form: FPDF_FORMHANDLE,
        annot: FPDF_ANNOTATION,
        buffer: *mut FPDF_WCHAR,
        buflen: c_ulong,
    ) -> c_ulong {
        unsafe { (self.extern_FPDFAnnot_GetFormFieldValue)(form, annot, buffer, buflen) }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FPDFAnnot_GetOptionCount(&self, form: FPDF_FORMHANDLE, annot: FPDF_ANNOTATION) -> c_int {
        unsafe { (self.extern_FPDFAnnot_GetOptionCount)(form, annot) }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FPDFAnnot_GetOptionLabel(
        &self,
        form: FPDF_FORMHANDLE,
        annot: FPDF_ANNOTATION,
        index: c_int,
        buffer: *mut FPDF_WCHAR,
        buflen: c_ulong,
    ) -> c_ulong {
        unsafe { (self.extern_FPDFAnnot_GetOptionLabel)(form, annot, index, buffer, buflen) }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FPDFAnnot_IsOptionSelected(&self, handle: FPDF_FORMHANDLE, annot: FPDF_ANNOTATION, index: c_int) -> FPDF_BOOL {
        unsafe { (self.extern_FPDFAnnot_IsOptionSelected)(handle, annot, index) }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FPDFAnnot_GetFontSize(&self, form: FPDF_FORMHANDLE, annot: FPDF_ANNOTATION, value: *mut f32) -> FPDF_BOOL {
        unsafe { (self.extern_FPDFAnnot_GetFontSize)(form, annot, value) }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FPDFAnnot_SetFontColor(
        &self,
        handle: FPDF_FORMHANDLE,
        annot: FPDF_ANNOTATION,
        R: c_uint,
        G: c_uint,
        B: c_uint,
    ) -> FPDF_BOOL {
        unsafe { (self.extern_FPDFAnnot_SetFontColor)(handle, annot, R, G, B) }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FPDFAnnot_GetFontColor(
        &self,
        form: FPDF_FORMHANDLE,
        annot: FPDF_ANNOTATION,
        R: *mut c_uint,
        G: *mut c_uint,
        B: *mut c_uint,
    ) -> FPDF_BOOL {
        unsafe { (self.extern_FPDFAnnot_GetFontColor)(form, annot, R, G, B) }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FPDFAnnot_IsChecked(&self, form: FPDF_FORMHANDLE, annot: FPDF_ANNOTATION) -> FPDF_BOOL {
        unsafe { (self.extern_FPDFAnnot_IsChecked)(form, annot) }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FPDFAnnot_SetFocusableSubtypes(
        &self,
        form: FPDF_FORMHANDLE,
        subtypes: *const FPDF_ANNOTATION_SUBTYPE,
        count: size_t,
    ) -> FPDF_BOOL {
        unsafe { (self.extern_FPDFAnnot_SetFocusableSubtypes)(form, subtypes, count) }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FPDFAnnot_GetFocusableSubtypesCount(&self, form: FPDF_FORMHANDLE) -> c_int {
        unsafe { (self.extern_FPDFAnnot_GetFocusableSubtypesCount)(form) }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FPDFAnnot_GetFocusableSubtypes(
        &self,
        form: FPDF_FORMHANDLE,
        subtypes: *mut FPDF_ANNOTATION_SUBTYPE,
        count: size_t,
    ) -> FPDF_BOOL {
        unsafe { (self.extern_FPDFAnnot_GetFocusableSubtypes)(form, subtypes, count) }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FPDFAnnot_GetLink(&self, annot: FPDF_ANNOTATION) -> FPDF_LINK {
        unsafe { (self.extern_FPDFAnnot_GetLink)(annot) }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FPDFAnnot_GetFormControlCount(&self, form: FPDF_FORMHANDLE, annot: FPDF_ANNOTATION) -> c_int {
        unsafe { (self.extern_FPDFAnnot_GetFormControlCount)(form, annot) }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FPDFAnnot_GetFormControlIndex(&self, form: FPDF_FORMHANDLE, annot: FPDF_ANNOTATION) -> c_int {
        unsafe { (self.extern_FPDFAnnot_GetFormControlIndex)(form, annot) }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FPDFAnnot_GetFormFieldExportValue(
        &self,
        form: FPDF_FORMHANDLE,
        annot: FPDF_ANNOTATION,
        buffer: *mut FPDF_WCHAR,
        buflen: c_ulong,
    ) -> c_ulong {
        unsafe { (self.extern_FPDFAnnot_GetFormFieldExportValue)(form, annot, buffer, buflen) }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FPDFAnnot_SetURI(&self, annot: FPDF_ANNOTATION, uri: &str) -> FPDF_BOOL {
        let c_uri = safe_cstring(uri);

        unsafe { (self.extern_FPDFAnnot_SetURI)(annot, c_uri.as_ptr()) }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FPDFAnnot_GetFileAttachment(&self, annot: FPDF_ANNOTATION) -> FPDF_ATTACHMENT {
        unsafe { (self.extern_FPDFAnnot_GetFileAttachment)(annot) }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FPDFAnnot_AddFileAttachment(&self, annot: FPDF_ANNOTATION, name: FPDF_WIDESTRING) -> FPDF_ATTACHMENT {
        unsafe { (self.extern_FPDFAnnot_AddFileAttachment)(annot, name) }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FPDFDOC_InitFormFillEnvironment(
        &self,
        document: FPDF_DOCUMENT,
        form_info: *mut FPDF_FORMFILLINFO,
    ) -> FPDF_FORMHANDLE {
        unsafe { (self.extern_FPDFDOC_InitFormFillEnvironment)(document, form_info) }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FPDFDOC_ExitFormFillEnvironment(&self, handle: FPDF_FORMHANDLE) {
        unsafe {
            (self.extern_FPDFDOC_ExitFormFillEnvironment)(handle);
        }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FORM_OnAfterLoadPage(&self, page: FPDF_PAGE, handle: FPDF_FORMHANDLE) {
        unsafe {
            (self.extern_FORM_OnAfterLoadPage)(page, handle);
        }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FORM_OnBeforeClosePage(&self, page: FPDF_PAGE, handle: FPDF_FORMHANDLE) {
        unsafe {
            (self.extern_FORM_OnBeforeClosePage)(page, handle);
        }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FPDFDoc_GetPageMode(&self, document: FPDF_DOCUMENT) -> c_int {
        unsafe { (self.extern_FPDFDoc_GetPageMode)(document) }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FPDFPage_Flatten(&self, page: FPDF_PAGE, nFlag: c_int) -> c_int {
        unsafe { (self.extern_FPDFPage_Flatten)(page, nFlag) }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FORM_DoDocumentJSAction(&self, form: FPDF_FORMHANDLE) {
        unsafe { (self.extern_FORM_DoDocumentJSAction)(form) }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FORM_DoDocumentOpenAction(&self, form: FPDF_FORMHANDLE) {
        unsafe { (self.extern_FORM_DoDocumentOpenAction)(form) }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FORM_DoDocumentAAction(&self, form: FPDF_FORMHANDLE, aaType: c_int) {
        unsafe { (self.extern_FORM_DoDocumentAAction)(form, aaType) }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FORM_DoPageAAction(&self, page: FPDF_PAGE, form: FPDF_FORMHANDLE, aaType: c_int) {
        unsafe { (self.extern_FORM_DoPageAAction)(page, form, aaType) }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FORM_OnMouseMove(
        &self,
        form: FPDF_FORMHANDLE,
        page: FPDF_PAGE,
        modifier: c_int,
        page_x: f64,
        page_y: f64,
    ) -> FPDF_BOOL {
        unsafe { (self.extern_FORM_OnMouseMove)(form, page, modifier, page_x, page_y) }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FORM_OnMouseWheel(
        &self,
        form: FPDF_FORMHANDLE,
        page: FPDF_PAGE,
        modifier: c_int,
        page_coord: *const FS_POINTF,
        delta_x: c_int,
        delta_y: c_int,
    ) -> FPDF_BOOL {
        unsafe { (self.extern_FORM_OnMouseWheel)(form, page, modifier, page_coord, delta_x, delta_y) }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FORM_OnFocus(
        &self,
        form: FPDF_FORMHANDLE,
        page: FPDF_PAGE,
        modifier: c_int,
        page_x: f64,
        page_y: f64,
    ) -> FPDF_BOOL {
        unsafe { (self.extern_FORM_OnFocus)(form, page, modifier, page_x, page_y) }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FORM_OnLButtonDown(
        &self,
        form: FPDF_FORMHANDLE,
        page: FPDF_PAGE,
        modifier: c_int,
        page_x: f64,
        page_y: f64,
    ) -> FPDF_BOOL {
        unsafe { (self.extern_FORM_OnLButtonDown)(form, page, modifier, page_x, page_y) }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FORM_OnRButtonDown(
        &self,
        form: FPDF_FORMHANDLE,
        page: FPDF_PAGE,
        modifier: c_int,
        page_x: f64,
        page_y: f64,
    ) -> FPDF_BOOL {
        unsafe { (self.extern_FORM_OnRButtonDown)(form, page, modifier, page_x, page_y) }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FORM_OnLButtonUp(
        &self,
        form: FPDF_FORMHANDLE,
        page: FPDF_PAGE,
        modifier: c_int,
        page_x: f64,
        page_y: f64,
    ) -> FPDF_BOOL {
        unsafe { (self.extern_FORM_OnLButtonUp)(form, page, modifier, page_x, page_y) }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FORM_OnRButtonUp(
        &self,
        form: FPDF_FORMHANDLE,
        page: FPDF_PAGE,
        modifier: c_int,
        page_x: f64,
        page_y: f64,
    ) -> FPDF_BOOL {
        unsafe { (self.extern_FORM_OnRButtonUp)(form, page, modifier, page_x, page_y) }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FORM_OnLButtonDoubleClick(
        &self,
        form: FPDF_FORMHANDLE,
        page: FPDF_PAGE,
        modifier: c_int,
        page_x: f64,
        page_y: f64,
    ) -> FPDF_BOOL {
        unsafe { (self.extern_FORM_OnLButtonDoubleClick)(form, page, modifier, page_x, page_y) }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FORM_OnKeyDown(&self, form: FPDF_FORMHANDLE, page: FPDF_PAGE, nKeyCode: c_int, modifier: c_int) -> FPDF_BOOL {
        unsafe { (self.extern_FORM_OnKeyDown)(form, page, nKeyCode, modifier) }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FORM_OnKeyUp(&self, form: FPDF_FORMHANDLE, page: FPDF_PAGE, nKeyCode: c_int, modifier: c_int) -> FPDF_BOOL {
        unsafe { (self.extern_FORM_OnKeyUp)(form, page, nKeyCode, modifier) }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FORM_OnChar(&self, form: FPDF_FORMHANDLE, page: FPDF_PAGE, nChar: c_int, modifier: c_int) -> FPDF_BOOL {
        unsafe { (self.extern_FORM_OnChar)(form, page, nChar, modifier) }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FORM_GetFocusedText(
        &self,
        form: FPDF_FORMHANDLE,
        page: FPDF_PAGE,
        buffer: *mut c_void,
        buflen: c_ulong,
    ) -> c_ulong {
        unsafe { (self.extern_FORM_GetFocusedText)(form, page, buffer, buflen) }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FORM_GetSelectedText(
        &self,
        form: FPDF_FORMHANDLE,
        page: FPDF_PAGE,
        buffer: *mut c_void,
        buflen: c_ulong,
    ) -> c_ulong {
        unsafe { (self.extern_FORM_GetSelectedText)(form, page, buffer, buflen) }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FORM_ReplaceAndKeepSelection(&self, form: FPDF_FORMHANDLE, page: FPDF_PAGE, wsText: FPDF_WIDESTRING) {
        unsafe { (self.extern_FORM_ReplaceAndKeepSelection)(form, page, wsText) }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FORM_ReplaceSelection(&self, form: FPDF_FORMHANDLE, page: FPDF_PAGE, wsText: FPDF_WIDESTRING) {
        unsafe { (self.extern_FORM_ReplaceSelection)(form, page, wsText) }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FORM_SelectAllText(&self, form: FPDF_FORMHANDLE, page: FPDF_PAGE) -> FPDF_BOOL {
        unsafe { (self.extern_FORM_SelectAllText)(form, page) }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FORM_CanUndo(&self, form: FPDF_FORMHANDLE, page: FPDF_PAGE) -> FPDF_BOOL {
        unsafe { (self.extern_FORM_CanUndo)(form, page) }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FORM_CanRedo(&self, form: FPDF_FORMHANDLE, page: FPDF_PAGE) -> FPDF_BOOL {
        unsafe { (self.extern_FORM_CanRedo)(form, page) }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FORM_Undo(&self, form: FPDF_FORMHANDLE, page: FPDF_PAGE) -> FPDF_BOOL {
        unsafe { (self.extern_FORM_Undo)(form, page) }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FORM_Redo(&self, form: FPDF_FORMHANDLE, page: FPDF_PAGE) -> FPDF_BOOL {
        unsafe { (self.extern_FORM_Redo)(form, page) }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FORM_ForceToKillFocus(&self, form: FPDF_FORMHANDLE) -> FPDF_BOOL {
        unsafe { (self.extern_FORM_ForceToKillFocus)(form) }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FORM_GetFocusedAnnot(
        &self,
        form: FPDF_FORMHANDLE,
        page_index: *mut c_int,
        annot: *mut FPDF_ANNOTATION,
    ) -> FPDF_BOOL {
        unsafe { (self.extern_FORM_GetFocusedAnnot)(form, page_index, annot) }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FORM_SetFocusedAnnot(&self, form: FPDF_FORMHANDLE, annot: FPDF_ANNOTATION) -> FPDF_BOOL {
        unsafe { (self.extern_FORM_SetFocusedAnnot)(form, annot) }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FPDFPage_HasFormFieldAtPoint(&self, form: FPDF_FORMHANDLE, page: FPDF_PAGE, page_x: f64, page_y: f64) -> c_int {
        unsafe { (self.extern_FPDFPage_HasFormFieldAtPoint)(form, page, page_x, page_y) }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FPDFPage_FormFieldZOrderAtPoint(
        &self,
        form: FPDF_FORMHANDLE,
        page: FPDF_PAGE,
        page_x: f64,
        page_y: f64,
    ) -> c_int {
        unsafe { (self.extern_FPDFPage_FormFieldZOrderAtPoint)(form, page, page_x, page_y) }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FPDF_SetFormFieldHighlightColor(&self, form: FPDF_FORMHANDLE, field_type: c_int, color: FPDF_DWORD) {
        unsafe {
            (self.extern_FPDF_SetFormFieldHighlightColor)(form, field_type, color);
        }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FPDF_SetFormFieldHighlightAlpha(&self, form: FPDF_FORMHANDLE, alpha: c_uchar) {
        unsafe {
            (self.extern_FPDF_SetFormFieldHighlightAlpha)(form, alpha);
        }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FPDF_RemoveFormFieldHighlight(&self, form: FPDF_FORMHANDLE) {
        unsafe { (self.extern_FPDF_RemoveFormFieldHighlight)(form) }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FPDF_FFLDraw(
        &self,
        form: FPDF_FORMHANDLE,
        bitmap: FPDF_BITMAP,
        page: FPDF_PAGE,
        start_x: c_int,
        start_y: c_int,
        size_x: c_int,
        size_y: c_int,
        rotate: c_int,
        flags: c_int,
    ) {
        unsafe {
            (self.extern_FPDF_FFLDraw)(form, bitmap, page, start_x, start_y, size_x, size_y, rotate, flags);
        }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FPDF_GetFormType(&self, document: FPDF_DOCUMENT) -> c_int {
        unsafe { (self.extern_FPDF_GetFormType)(document) }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FORM_SetIndexSelected(
        &self,
        form: FPDF_FORMHANDLE,
        page: FPDF_PAGE,
        index: c_int,
        selected: FPDF_BOOL,
    ) -> FPDF_BOOL {
        unsafe { (self.extern_FORM_SetIndexSelected)(form, page, index, selected) }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FORM_IsIndexSelected(&self, form: FPDF_FORMHANDLE, page: FPDF_PAGE, index: c_int) -> FPDF_BOOL {
        unsafe { (self.extern_FORM_IsIndexSelected)(form, page, index) }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FPDF_LoadXFA(&self, document: FPDF_DOCUMENT) -> FPDF_BOOL {
        unsafe { (self.extern_FPDF_LoadXFA)(document) }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FPDFDoc_GetJavaScriptActionCount(&self, document: FPDF_DOCUMENT) -> c_int {
        unsafe { (self.extern_FPDFDoc_GetJavaScriptActionCount)(document) }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FPDFDoc_GetJavaScriptAction(&self, document: FPDF_DOCUMENT, index: c_int) -> FPDF_JAVASCRIPT_ACTION {
        unsafe { (self.extern_FPDFDoc_GetJavaScriptAction)(document, index) }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FPDFDoc_CloseJavaScriptAction(&self, javascript: FPDF_JAVASCRIPT_ACTION) {
        unsafe { (self.extern_FPDFDoc_CloseJavaScriptAction)(javascript) }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FPDFJavaScriptAction_GetName(
        &self,
        javascript: FPDF_JAVASCRIPT_ACTION,
        buffer: *mut FPDF_WCHAR,
        buflen: c_ulong,
    ) -> c_ulong {
        unsafe { (self.extern_FPDFJavaScriptAction_GetName)(javascript, buffer, buflen) }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FPDFJavaScriptAction_GetScript(
        &self,
        javascript: FPDF_JAVASCRIPT_ACTION,
        buffer: *mut FPDF_WCHAR,
        buflen: c_ulong,
    ) -> c_ulong {
        unsafe { (self.extern_FPDFJavaScriptAction_GetScript)(javascript, buffer, buflen) }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FPDF_GetDefaultTTFMap(&self) -> *const FPDF_CharsetFontMap {
        unsafe { (self.extern_FPDF_GetDefaultTTFMap)() }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FPDF_GetDefaultTTFMapCount(&self) -> usize {
        unsafe { (self.extern_FPDF_GetDefaultTTFMapCount)() }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FPDF_GetDefaultTTFMapEntry(&self, index: usize) -> *const FPDF_CharsetFontMap {
        unsafe { (self.extern_FPDF_GetDefaultTTFMapEntry)(index) }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FPDF_AddInstalledFont(&self, mapper: *mut c_void, face: &str, charset: c_int) {
        let c_face = safe_cstring(face);

        unsafe { (self.extern_FPDF_AddInstalledFont)(mapper, c_face.as_ptr(), charset) }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FPDF_SetSystemFontInfo(&self, pFontInfo: *mut FPDF_SYSFONTINFO) {
        unsafe { (self.extern_FPDF_SetSystemFontInfo)(pFontInfo) }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FPDF_GetDefaultSystemFontInfo(&self) -> *mut FPDF_SYSFONTINFO {
        unsafe { (self.extern_FPDF_GetDefaultSystemFontInfo)() }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FPDF_FreeDefaultSystemFontInfo(&self, pFontInfo: *mut FPDF_SYSFONTINFO) {
        unsafe { (self.extern_FPDF_FreeDefaultSystemFontInfo)(pFontInfo) }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FPDFBookmark_GetFirstChild(&self, document: FPDF_DOCUMENT, bookmark: FPDF_BOOKMARK) -> FPDF_BOOKMARK {
        unsafe { (self.extern_FPDFBookmark_GetFirstChild)(document, bookmark) }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FPDFBookmark_GetNextSibling(&self, document: FPDF_DOCUMENT, bookmark: FPDF_BOOKMARK) -> FPDF_BOOKMARK {
        unsafe { (self.extern_FPDFBookmark_GetNextSibling)(document, bookmark) }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FPDFBookmark_GetTitle(&self, bookmark: FPDF_BOOKMARK, buffer: *mut c_void, buflen: c_ulong) -> c_ulong {
        unsafe { (self.extern_FPDFBookmark_GetTitle)(bookmark, buffer, buflen) }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FPDFBookmark_GetCount(&self, bookmark: FPDF_BOOKMARK) -> c_int {
        unsafe { (self.extern_FPDFBookmark_GetCount)(bookmark) }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FPDFBookmark_Find(&self, document: FPDF_DOCUMENT, title: FPDF_WIDESTRING) -> FPDF_BOOKMARK {
        unsafe { (self.extern_FPDFBookmark_Find)(document, title) }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FPDFBookmark_GetDest(&self, document: FPDF_DOCUMENT, bookmark: FPDF_BOOKMARK) -> FPDF_DEST {
        unsafe { (self.extern_FPDFBookmark_GetDest)(document, bookmark) }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FPDFBookmark_GetAction(&self, bookmark: FPDF_BOOKMARK) -> FPDF_ACTION {
        unsafe { (self.extern_FPDFBookmark_GetAction)(bookmark) }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FPDFAction_GetType(&self, action: FPDF_ACTION) -> c_ulong {
        unsafe { (self.extern_FPDFAction_GetType)(action) }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FPDFAction_GetDest(&self, document: FPDF_DOCUMENT, action: FPDF_ACTION) -> FPDF_DEST {
        unsafe { (self.extern_FPDFAction_GetDest)(document, action) }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FPDFAction_GetFilePath(&self, action: FPDF_ACTION, buffer: *mut c_void, buflen: c_ulong) -> c_ulong {
        unsafe { (self.extern_FPDFAction_GetFilePath)(action, buffer, buflen) }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FPDFAction_GetURIPath(
        &self,
        document: FPDF_DOCUMENT,
        action: FPDF_ACTION,
        buffer: *mut c_void,
        buflen: c_ulong,
    ) -> c_ulong {
        unsafe { (self.extern_FPDFAction_GetURIPath)(document, action, buffer, buflen) }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FPDFDest_GetDestPageIndex(&self, document: FPDF_DOCUMENT, dest: FPDF_DEST) -> c_int {
        unsafe { (self.extern_FPDFDest_GetDestPageIndex)(document, dest) }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FPDFDest_GetView(&self, dest: FPDF_DEST, pNumParams: *mut c_ulong, pParams: *mut FS_FLOAT) -> c_ulong {
        unsafe { (self.extern_FPDFDest_GetView)(dest, pNumParams, pParams) }
    }

    #[inline]
    #[allow(non_snake_case)]
    #[allow(clippy::too_many_arguments)]
    fn FPDFDest_GetLocationInPage(
        &self,
        dest: FPDF_DEST,
        hasXVal: *mut FPDF_BOOL,
        hasYVal: *mut FPDF_BOOL,
        hasZoomVal: *mut FPDF_BOOL,
        x: *mut FS_FLOAT,
        y: *mut FS_FLOAT,
        zoom: *mut FS_FLOAT,
    ) -> FPDF_BOOL {
        unsafe { (self.extern_FPDFDest_GetLocationInPage)(dest, hasXVal, hasYVal, hasZoomVal, x, y, zoom) }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FPDFLink_GetLinkAtPoint(&self, page: FPDF_PAGE, x: c_double, y: c_double) -> FPDF_LINK {
        unsafe { (self.extern_FPDFLink_GetLinkAtPoint)(page, x, y) }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FPDFLink_GetLinkZOrderAtPoint(&self, page: FPDF_PAGE, x: c_double, y: c_double) -> c_int {
        unsafe { (self.extern_FPDFLink_GetLinkZOrderAtPoint)(page, x, y) }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FPDFLink_GetDest(&self, document: FPDF_DOCUMENT, link: FPDF_LINK) -> FPDF_DEST {
        unsafe { (self.extern_FPDFLink_GetDest)(document, link) }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FPDFLink_GetAction(&self, link: FPDF_LINK) -> FPDF_ACTION {
        unsafe { (self.extern_FPDFLink_GetAction)(link) }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FPDFLink_Enumerate(&self, page: FPDF_PAGE, start_pos: *mut c_int, link_annot: *mut FPDF_LINK) -> FPDF_BOOL {
        unsafe { (self.extern_FPDFLink_Enumerate)(page, start_pos, link_annot) }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FPDFLink_GetAnnot(&self, page: FPDF_PAGE, link_annot: FPDF_LINK) -> FPDF_ANNOTATION {
        unsafe { (self.extern_FPDFLink_GetAnnot)(page, link_annot) }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FPDFLink_GetAnnotRect(&self, link_annot: FPDF_LINK, rect: *mut FS_RECTF) -> FPDF_BOOL {
        unsafe { (self.extern_FPDFLink_GetAnnotRect)(link_annot, rect) }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FPDFLink_CountQuadPoints(&self, link_annot: FPDF_LINK) -> c_int {
        unsafe { (self.extern_FPDFLink_CountQuadPoints)(link_annot) }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FPDFLink_GetQuadPoints(
        &self,
        link_annot: FPDF_LINK,
        quad_index: c_int,
        quad_points: *mut FS_QUADPOINTSF,
    ) -> FPDF_BOOL {
        unsafe { (self.extern_FPDFLink_GetQuadPoints)(link_annot, quad_index, quad_points) }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FPDF_GetPageAAction(&self, page: FPDF_PAGE, aa_type: c_int) -> FPDF_ACTION {
        unsafe { (self.extern_FPDF_GetPageAAction)(page, aa_type) }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FPDFText_LoadPage(&self, page: FPDF_PAGE) -> FPDF_TEXTPAGE {
        unsafe { (self.extern_FPDFText_LoadPage)(page) }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FPDFText_ClosePage(&self, text_page: FPDF_TEXTPAGE) {
        unsafe {
            (self.extern_FPDFText_ClosePage)(text_page);
        }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FPDFText_CountChars(&self, text_page: FPDF_TEXTPAGE) -> c_int {
        unsafe { (self.extern_FPDFText_CountChars)(text_page) }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FPDFText_GetUnicode(&self, text_page: FPDF_TEXTPAGE, index: c_int) -> c_uint {
        unsafe { (self.extern_FPDFText_GetUnicode)(text_page, index) }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FPDFText_GetTextObject(&self, text_page: FPDF_TEXTPAGE, index: c_int) -> FPDF_PAGEOBJECT {
        unsafe { (self.extern_FPDFText_GetTextObject)(text_page, index) }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FPDFText_IsGenerated(&self, text_page: FPDF_TEXTPAGE, index: c_int) -> c_int {
        unsafe { (self.extern_FPDFText_IsGenerated)(text_page, index) }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FPDFText_IsHyphen(&self, text_page: FPDF_TEXTPAGE, index: c_int) -> c_int {
        unsafe { (self.extern_FPDFText_IsHyphen)(text_page, index) }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FPDFText_HasUnicodeMapError(&self, text_page: FPDF_TEXTPAGE, index: c_int) -> c_int {
        unsafe { (self.extern_FPDFText_HasUnicodeMapError)(text_page, index) }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FPDFText_GetFontSize(&self, text_page: FPDF_TEXTPAGE, index: c_int) -> c_double {
        unsafe { (self.extern_FPDFText_GetFontSize)(text_page, index) }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FPDFText_GetFontInfo(
        &self,
        text_page: FPDF_TEXTPAGE,
        index: c_int,
        buffer: *mut c_void,
        buflen: c_ulong,
        flags: *mut c_int,
    ) -> c_ulong {
        unsafe { (self.extern_FPDFText_GetFontInfo)(text_page, index, buffer, buflen, flags) }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FPDFText_GetFontWeight(&self, text_page: FPDF_TEXTPAGE, index: c_int) -> c_int {
        unsafe { (self.extern_FPDFText_GetFontWeight)(text_page, index) }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FPDFText_GetFillColor(
        &self,
        text_page: FPDF_TEXTPAGE,
        index: c_int,
        R: *mut c_uint,
        G: *mut c_uint,
        B: *mut c_uint,
        A: *mut c_uint,
    ) -> FPDF_BOOL {
        unsafe { (self.extern_FPDFText_GetFillColor)(text_page, index, R, G, B, A) }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FPDFText_GetStrokeColor(
        &self,
        text_page: FPDF_TEXTPAGE,
        index: c_int,
        R: *mut c_uint,
        G: *mut c_uint,
        B: *mut c_uint,
        A: *mut c_uint,
    ) -> FPDF_BOOL {
        unsafe { (self.extern_FPDFText_GetStrokeColor)(text_page, index, R, G, B, A) }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FPDFText_GetCharAngle(&self, text_page: FPDF_TEXTPAGE, index: c_int) -> c_float {
        unsafe { (self.extern_FPDFText_GetCharAngle)(text_page, index) }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FPDFText_GetCharBox(
        &self,
        text_page: FPDF_TEXTPAGE,
        index: c_int,
        left: *mut c_double,
        right: *mut c_double,
        bottom: *mut c_double,
        top: *mut c_double,
    ) -> FPDF_BOOL {
        unsafe { (self.extern_FPDFText_GetCharBox)(text_page, index, left, right, bottom, top) }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FPDFText_GetLooseCharBox(&self, text_page: FPDF_TEXTPAGE, index: c_int, rect: *mut FS_RECTF) -> FPDF_BOOL {
        unsafe { (self.extern_FPDFText_GetLooseCharBox)(text_page, index, rect) }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FPDFText_GetMatrix(&self, text_page: FPDF_TEXTPAGE, index: c_int, matrix: *mut FS_MATRIX) -> FPDF_BOOL {
        unsafe { (self.extern_FPDFText_GetMatrix)(text_page, index, matrix) }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FPDFText_GetCharOrigin(
        &self,
        text_page: FPDF_TEXTPAGE,
        index: c_int,
        x: *mut c_double,
        y: *mut c_double,
    ) -> FPDF_BOOL {
        unsafe { (self.extern_FPDFText_GetCharOrigin)(text_page, index, x, y) }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FPDFText_GetCharIndexAtPos(
        &self,
        text_page: FPDF_TEXTPAGE,
        x: c_double,
        y: c_double,
        xTolerance: c_double,
        yTolerance: c_double,
    ) -> c_int {
        unsafe { (self.extern_FPDFText_GetCharIndexAtPos)(text_page, x, y, xTolerance, yTolerance) }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FPDFText_GetText(
        &self,
        text_page: FPDF_TEXTPAGE,
        start_index: c_int,
        count: c_int,
        result: *mut c_ushort,
    ) -> c_int {
        unsafe { (self.extern_FPDFText_GetText)(text_page, start_index, count, result) }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FPDFText_CountRects(&self, text_page: FPDF_TEXTPAGE, start_index: c_int, count: c_int) -> c_int {
        unsafe { (self.extern_FPDFText_CountRects)(text_page, start_index, count) }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FPDFText_GetRect(
        &self,
        text_page: FPDF_TEXTPAGE,
        rect_index: c_int,
        left: *mut c_double,
        top: *mut c_double,
        right: *mut c_double,
        bottom: *mut c_double,
    ) -> FPDF_BOOL {
        unsafe { (self.extern_FPDFText_GetRect)(text_page, rect_index, left, top, right, bottom) }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FPDFText_GetBoundedText(
        &self,
        text_page: FPDF_TEXTPAGE,
        left: c_double,
        top: c_double,
        right: c_double,
        bottom: c_double,
        buffer: *mut c_ushort,
        buflen: c_int,
    ) -> c_int {
        unsafe { (self.extern_FPDFText_GetBoundedText)(text_page, left, top, right, bottom, buffer, buflen) }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FPDFText_FindStart(
        &self,
        text_page: FPDF_TEXTPAGE,
        findwhat: FPDF_WIDESTRING,
        flags: c_ulong,
        start_index: c_int,
    ) -> FPDF_SCHHANDLE {
        unsafe { (self.extern_FPDFText_FindStart)(text_page, findwhat, flags, start_index) }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FPDFText_FindNext(&self, handle: FPDF_SCHHANDLE) -> FPDF_BOOL {
        unsafe { (self.extern_FPDFText_FindNext)(handle) }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FPDFText_FindPrev(&self, handle: FPDF_SCHHANDLE) -> FPDF_BOOL {
        unsafe { (self.extern_FPDFText_FindPrev)(handle) }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FPDFText_GetSchResultIndex(&self, handle: FPDF_SCHHANDLE) -> c_int {
        unsafe { (self.extern_FPDFText_GetSchResultIndex)(handle) }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FPDFText_GetSchCount(&self, handle: FPDF_SCHHANDLE) -> c_int {
        unsafe { (self.extern_FPDFText_GetSchCount)(handle) }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FPDFText_FindClose(&self, handle: FPDF_SCHHANDLE) {
        unsafe { (self.extern_FPDFText_FindClose)(handle) }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FPDFLink_LoadWebLinks(&self, text_page: FPDF_TEXTPAGE) -> FPDF_PAGELINK {
        unsafe { (self.extern_FPDFLink_LoadWebLinks)(text_page) }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FPDFLink_CountWebLinks(&self, link_page: FPDF_PAGELINK) -> c_int {
        unsafe { (self.extern_FPDFLink_CountWebLinks)(link_page) }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FPDFLink_GetURL(
        &self,
        link_page: FPDF_PAGELINK,
        link_index: c_int,
        buffer: *mut c_ushort,
        buflen: c_int,
    ) -> c_int {
        unsafe { (self.extern_FPDFLink_GetURL)(link_page, link_index, buffer, buflen) }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FPDFLink_CountRects(&self, link_page: FPDF_PAGELINK, link_index: c_int) -> c_int {
        unsafe { (self.extern_FPDFLink_CountRects)(link_page, link_index) }
    }

    #[inline]
    #[allow(non_snake_case)]
    #[allow(clippy::too_many_arguments)]
    fn FPDFLink_GetRect(
        &self,
        link_page: FPDF_PAGELINK,
        link_index: c_int,
        rect_index: c_int,
        left: *mut c_double,
        top: *mut c_double,
        right: *mut c_double,
        bottom: *mut c_double,
    ) -> FPDF_BOOL {
        unsafe { (self.extern_FPDFLink_GetRect)(link_page, link_index, rect_index, left, top, right, bottom) }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FPDFLink_GetTextRange(
        &self,
        link_page: FPDF_PAGELINK,
        link_index: c_int,
        start_char_index: *mut c_int,
        char_count: *mut c_int,
    ) -> FPDF_BOOL {
        unsafe { (self.extern_FPDFLink_GetTextRange)(link_page, link_index, start_char_index, char_count) }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FPDFLink_CloseWebLinks(&self, link_page: FPDF_PAGELINK) {
        unsafe { (self.extern_FPDFLink_CloseWebLinks)(link_page) }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FPDFPage_GetDecodedThumbnailData(&self, page: FPDF_PAGE, buffer: *mut c_void, buflen: c_ulong) -> c_ulong {
        unsafe { (self.extern_FPDFPage_GetDecodedThumbnailData)(page, buffer, buflen) }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FPDFPage_GetRawThumbnailData(&self, page: FPDF_PAGE, buffer: *mut c_void, buflen: c_ulong) -> c_ulong {
        unsafe { (self.extern_FPDFPage_GetRawThumbnailData)(page, buffer, buflen) }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FPDFPage_GetThumbnailAsBitmap(&self, page: FPDF_PAGE) -> FPDF_BITMAP {
        unsafe { (self.extern_FPDFPage_GetThumbnailAsBitmap)(page) }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FPDFFormObj_CountObjects(&self, form_object: FPDF_PAGEOBJECT) -> c_int {
        unsafe { (self.extern_FPDFFormObj_CountObjects)(form_object) }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FPDFFormObj_GetObject(&self, form_object: FPDF_PAGEOBJECT, index: c_ulong) -> FPDF_PAGEOBJECT {
        unsafe { (self.extern_FPDFFormObj_GetObject)(form_object, index) }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FPDFFormObj_RemoveObject(&self, form_object: FPDF_PAGEOBJECT, page_object: FPDF_PAGEOBJECT) -> FPDF_BOOL {
        unsafe { (self.extern_FPDFFormObj_RemoveObject)(form_object, page_object) }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FPDFPageObj_CreateTextObj(
        &self,
        document: FPDF_DOCUMENT,
        font: FPDF_FONT,
        font_size: c_float,
    ) -> FPDF_PAGEOBJECT {
        unsafe { (self.extern_FPDFPageObj_CreateTextObj)(document, font, font_size) }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FPDFTextObj_GetTextRenderMode(&self, text: FPDF_PAGEOBJECT) -> FPDF_TEXT_RENDERMODE {
        unsafe { (self.extern_FPDFTextObj_GetTextRenderMode)(text) }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FPDFTextObj_SetTextRenderMode(&self, text: FPDF_PAGEOBJECT, render_mode: FPDF_TEXT_RENDERMODE) -> FPDF_BOOL {
        unsafe { (self.extern_FPDFTextObj_SetTextRenderMode)(text, render_mode) }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FPDFTextObj_GetText(
        &self,
        text_object: FPDF_PAGEOBJECT,
        text_page: FPDF_TEXTPAGE,
        buffer: *mut FPDF_WCHAR,
        length: c_ulong,
    ) -> c_ulong {
        unsafe { (self.extern_FPDFTextObj_GetText)(text_object, text_page, buffer, length) }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FPDFTextObj_GetRenderedBitmap(
        &self,
        document: FPDF_DOCUMENT,
        page: FPDF_PAGE,
        text_object: FPDF_PAGEOBJECT,
        scale: f32,
    ) -> FPDF_BITMAP {
        unsafe { (self.extern_FPDFTextObj_GetRenderedBitmap)(document, page, text_object, scale) }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FPDFTextObj_GetFont(&self, text: FPDF_PAGEOBJECT) -> FPDF_FONT {
        unsafe { (self.extern_FPDFTextObj_GetFont)(text) }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FPDFTextObj_GetFontSize(&self, text: FPDF_PAGEOBJECT, size: *mut c_float) -> FPDF_BOOL {
        unsafe { (self.extern_FPDFTextObj_GetFontSize)(text, size) }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FPDFPageObj_NewTextObj(&self, document: FPDF_DOCUMENT, font: &str, font_size: c_float) -> FPDF_PAGEOBJECT {
        let c_font = safe_cstring(font);

        unsafe { (self.extern_FPDFPageObj_NewTextObj)(document, c_font.as_ptr(), font_size) }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FPDFText_SetText(&self, text_object: FPDF_PAGEOBJECT, text: FPDF_WIDESTRING) -> FPDF_BOOL {
        unsafe { (self.extern_FPDFText_SetText)(text_object, text) }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FPDFText_SetCharcodes(
        &self,
        text_object: FPDF_PAGEOBJECT,
        charcodes: *const c_uint,
        count: size_t,
    ) -> FPDF_BOOL {
        unsafe { (self.extern_FPDFText_SetCharcodes)(text_object, charcodes, count) }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FPDFText_LoadFont(
        &self,
        document: FPDF_DOCUMENT,
        data: *const c_uchar,
        size: c_uint,
        font_type: c_int,
        cid: FPDF_BOOL,
    ) -> FPDF_FONT {
        unsafe { (self.extern_FPDFText_LoadFont)(document, data, size, font_type, cid) }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FPDFText_LoadStandardFont(&self, document: FPDF_DOCUMENT, font: &str) -> FPDF_FONT {
        let c_font = safe_cstring(font);

        unsafe { (self.extern_FPDFText_LoadStandardFont)(document, c_font.as_ptr()) }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FPDFText_LoadCidType2Font(
        &self,
        document: FPDF_DOCUMENT,
        font_data: *const u8,
        font_data_size: u32,
        to_unicode_cmap: &str,
        cid_to_gid_map_data: *const u8,
        cid_to_gid_map_data_size: u32,
    ) -> FPDF_FONT {
        let c_to_unicode_cmap = safe_cstring(to_unicode_cmap);

        unsafe {
            (self.extern_FPDFText_LoadCidType2Font)(
                document,
                font_data,
                font_data_size,
                c_to_unicode_cmap.as_ptr(),
                cid_to_gid_map_data,
                cid_to_gid_map_data_size,
            )
        }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FPDFFont_Close(&self, font: FPDF_FONT) {
        unsafe { (self.extern_FPDFFont_Close)(font) }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FPDFPath_MoveTo(&self, path: FPDF_PAGEOBJECT, x: c_float, y: c_float) -> FPDF_BOOL {
        unsafe { (self.extern_FPDFPath_MoveTo)(path, x, y) }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FPDFPath_LineTo(&self, path: FPDF_PAGEOBJECT, x: c_float, y: c_float) -> FPDF_BOOL {
        unsafe { (self.extern_FPDFPath_LineTo)(path, x, y) }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FPDFPath_BezierTo(
        &self,
        path: FPDF_PAGEOBJECT,
        x1: c_float,
        y1: c_float,
        x2: c_float,
        y2: c_float,
        x3: c_float,
        y3: c_float,
    ) -> FPDF_BOOL {
        unsafe { (self.extern_FPDFPath_BezierTo)(path, x1, y1, x2, y2, x3, y3) }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FPDFPath_Close(&self, path: FPDF_PAGEOBJECT) -> FPDF_BOOL {
        unsafe { (self.extern_FPDFPath_Close)(path) }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FPDFPath_SetDrawMode(&self, path: FPDF_PAGEOBJECT, fillmode: c_int, stroke: FPDF_BOOL) -> FPDF_BOOL {
        unsafe { (self.extern_FPDFPath_SetDrawMode)(path, fillmode, stroke) }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FPDFPath_GetDrawMode(&self, path: FPDF_PAGEOBJECT, fillmode: *mut c_int, stroke: *mut FPDF_BOOL) -> FPDF_BOOL {
        unsafe { (self.extern_FPDFPath_GetDrawMode)(path, fillmode, stroke) }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FPDFPage_InsertObject(&self, page: FPDF_PAGE, page_obj: FPDF_PAGEOBJECT) {
        unsafe { (self.extern_FPDFPage_InsertObject)(page, page_obj) }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FPDFPage_InsertObjectAtIndex(&self, page: FPDF_PAGE, page_object: FPDF_PAGEOBJECT, index: usize) -> FPDF_BOOL {
        unsafe { (self.extern_FPDFPage_InsertObjectAtIndex)(page, page_object, index) }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FPDFPage_RemoveObject(&self, page: FPDF_PAGE, page_obj: FPDF_PAGEOBJECT) -> FPDF_BOOL {
        unsafe { (self.extern_FPDFPage_RemoveObject)(page, page_obj) }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FPDFPage_CountObjects(&self, page: FPDF_PAGE) -> c_int {
        unsafe { (self.extern_FPDFPage_CountObjects)(page) }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FPDFPage_GetObject(&self, page: FPDF_PAGE, index: c_int) -> FPDF_PAGEOBJECT {
        unsafe { (self.extern_FPDFPage_GetObject)(page, index) }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FPDFPageObj_Destroy(&self, page_obj: FPDF_PAGEOBJECT) {
        unsafe { (self.extern_FPDFPageObj_Destroy)(page_obj) }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FPDFPageObj_HasTransparency(&self, page_object: FPDF_PAGEOBJECT) -> FPDF_BOOL {
        unsafe { (self.extern_FPDFPageObj_HasTransparency)(page_object) }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FPDFPageObj_GetType(&self, page_object: FPDF_PAGEOBJECT) -> c_int {
        unsafe { (self.extern_FPDFPageObj_GetType)(page_object) }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FPDFPageObj_GetIsActive(&self, page_object: FPDF_PAGEOBJECT, active: *mut FPDF_BOOL) -> FPDF_BOOL {
        unsafe { (self.extern_FPDFPageObj_GetIsActive)(page_object, active) }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FPDFPageObj_SetIsActive(&self, page_object: FPDF_PAGEOBJECT, active: FPDF_BOOL) -> FPDF_BOOL {
        unsafe { (self.extern_FPDFPageObj_SetIsActive)(page_object, active) }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FPDFPageObj_Transform(
        &self,
        page_object: FPDF_PAGEOBJECT,
        a: c_double,
        b: c_double,
        c: c_double,
        d: c_double,
        e: c_double,
        f: c_double,
    ) {
        unsafe { (self.extern_FPDFPageObj_Transform)(page_object, a, b, c, d, e, f) }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FPDFPageObj_TransformF(&self, page_object: FPDF_PAGEOBJECT, matrix: *const FS_MATRIX) -> FPDF_BOOL {
        unsafe { (self.extern_FPDFPageObj_TransformF)(page_object, matrix) }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FPDFPageObj_GetMatrix(&self, page_object: FPDF_PAGEOBJECT, matrix: *mut FS_MATRIX) -> FPDF_BOOL {
        unsafe { (self.extern_FPDFPageObj_GetMatrix)(page_object, matrix) }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FPDFPageObj_SetMatrix(&self, path: FPDF_PAGEOBJECT, matrix: *const FS_MATRIX) -> FPDF_BOOL {
        unsafe { (self.extern_FPDFPageObj_SetMatrix)(path, matrix) }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FPDFPageObj_NewImageObj(&self, document: FPDF_DOCUMENT) -> FPDF_PAGEOBJECT {
        unsafe { (self.extern_FPDFPageObj_NewImageObj)(document) }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FPDFPageObj_GetMarkedContentID(&self, page_object: FPDF_PAGEOBJECT) -> c_int {
        unsafe { (self.extern_FPDFPageObj_GetMarkedContentID)(page_object) }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FPDFPageObj_CountMarks(&self, page_object: FPDF_PAGEOBJECT) -> c_int {
        unsafe { (self.extern_FPDFPageObj_CountMarks)(page_object) }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FPDFPageObj_GetMark(&self, page_object: FPDF_PAGEOBJECT, index: c_ulong) -> FPDF_PAGEOBJECTMARK {
        unsafe { (self.extern_FPDFPageObj_GetMark)(page_object, index) }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FPDFPageObj_AddMark(&self, page_object: FPDF_PAGEOBJECT, name: &str) -> FPDF_PAGEOBJECTMARK {
        let c_name = safe_cstring(name);

        unsafe { (self.extern_FPDFPageObj_AddMark)(page_object, c_name.as_ptr()) }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FPDFPageObj_RemoveMark(&self, page_object: FPDF_PAGEOBJECT, mark: FPDF_PAGEOBJECTMARK) -> FPDF_BOOL {
        unsafe { (self.extern_FPDFPageObj_RemoveMark)(page_object, mark) }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FPDFPageObjMark_GetName(
        &self,
        mark: FPDF_PAGEOBJECTMARK,
        buffer: *mut FPDF_WCHAR,
        buflen: c_ulong,
        out_buflen: *mut c_ulong,
    ) -> FPDF_BOOL {
        unsafe { (self.extern_FPDFPageObjMark_GetName)(mark, buffer, buflen, out_buflen) }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FPDFPageObjMark_CountParams(&self, mark: FPDF_PAGEOBJECTMARK) -> c_int {
        unsafe { (self.extern_FPDFPageObjMark_CountParams)(mark) }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FPDFPageObjMark_GetParamKey(
        &self,
        mark: FPDF_PAGEOBJECTMARK,
        index: c_ulong,
        buffer: *mut FPDF_WCHAR,
        buflen: c_ulong,
        out_buflen: *mut c_ulong,
    ) -> FPDF_BOOL {
        unsafe { (self.extern_FPDFPageObjMark_GetParamKey)(mark, index, buffer, buflen, out_buflen) }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FPDFPageObjMark_GetParamValueType(&self, mark: FPDF_PAGEOBJECTMARK, key: &str) -> FPDF_OBJECT_TYPE {
        let c_key = safe_cstring(key);

        unsafe { (self.extern_FPDFPageObjMark_GetParamValueType)(mark, c_key.as_ptr()) }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FPDFPageObjMark_GetParamIntValue(
        &self,
        mark: FPDF_PAGEOBJECTMARK,
        key: &str,
        out_value: *mut c_int,
    ) -> FPDF_BOOL {
        let c_key = safe_cstring(key);

        unsafe { (self.extern_FPDFPageObjMark_GetParamIntValue)(mark, c_key.as_ptr(), out_value) }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FPDFPageObjMark_GetParamFloatValue(
        &self,
        mark: FPDF_PAGEOBJECTMARK,
        key: &str,
        out_value: *mut f32,
    ) -> FPDF_BOOL {
        let c_key = safe_cstring(key);

        unsafe { (self.extern_FPDFPageObjMark_GetParamFloatValue)(mark, c_key.as_ptr(), out_value) }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FPDFPageObjMark_GetParamStringValue(
        &self,
        mark: FPDF_PAGEOBJECTMARK,
        key: &str,
        buffer: *mut FPDF_WCHAR,
        buflen: c_ulong,
        out_buflen: *mut c_ulong,
    ) -> FPDF_BOOL {
        let c_key = safe_cstring(key);

        unsafe { (self.extern_FPDFPageObjMark_GetParamStringValue)(mark, c_key.as_ptr(), buffer, buflen, out_buflen) }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FPDFPageObjMark_GetParamBlobValue(
        &self,
        mark: FPDF_PAGEOBJECTMARK,
        key: &str,
        buffer: *mut c_uchar,
        buflen: c_ulong,
        out_buflen: *mut c_ulong,
    ) -> FPDF_BOOL {
        let c_key = safe_cstring(key);

        unsafe { (self.extern_FPDFPageObjMark_GetParamBlobValue)(mark, c_key.as_ptr(), buffer, buflen, out_buflen) }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FPDFPageObjMark_SetIntParam(
        &self,
        document: FPDF_DOCUMENT,
        page_object: FPDF_PAGEOBJECT,
        mark: FPDF_PAGEOBJECTMARK,
        key: &str,
        value: c_int,
    ) -> FPDF_BOOL {
        let c_key = safe_cstring(key);

        unsafe { (self.extern_FPDFPageObjMark_SetIntParam)(document, page_object, mark, c_key.as_ptr(), value) }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FPDFPageObjMark_SetFloatParam(
        &self,
        document: FPDF_DOCUMENT,
        page_object: FPDF_PAGEOBJECT,
        mark: FPDF_PAGEOBJECTMARK,
        key: &str,
        value: f32,
    ) -> FPDF_BOOL {
        let c_key = safe_cstring(key);

        unsafe { (self.extern_FPDFPageObjMark_SetFloatParam)(document, page_object, mark, c_key.as_ptr(), value) }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FPDFPageObjMark_SetStringParam(
        &self,
        document: FPDF_DOCUMENT,
        page_object: FPDF_PAGEOBJECT,
        mark: FPDF_PAGEOBJECTMARK,
        key: &str,
        value: &str,
    ) -> FPDF_BOOL {
        let c_key = safe_cstring(key);

        let c_value = safe_cstring(value);

        unsafe {
            (self.extern_FPDFPageObjMark_SetStringParam)(document, page_object, mark, c_key.as_ptr(), c_value.as_ptr())
        }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FPDFPageObjMark_SetBlobParam(
        &self,
        document: FPDF_DOCUMENT,
        page_object: FPDF_PAGEOBJECT,
        mark: FPDF_PAGEOBJECTMARK,
        key: &str,
        value: *const c_uchar,
        value_len: c_ulong,
    ) -> FPDF_BOOL {
        let c_key = safe_cstring(key);

        unsafe {
            (self.extern_FPDFPageObjMark_SetBlobParam)(document, page_object, mark, c_key.as_ptr(), value, value_len)
        }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FPDFPageObjMark_RemoveParam(
        &self,
        page_object: FPDF_PAGEOBJECT,
        mark: FPDF_PAGEOBJECTMARK,
        key: &str,
    ) -> FPDF_BOOL {
        let c_key = safe_cstring(key);

        unsafe { (self.extern_FPDFPageObjMark_RemoveParam)(page_object, mark, c_key.as_ptr()) }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FPDFImageObj_LoadJpegFile(
        &self,
        pages: *mut FPDF_PAGE,
        count: c_int,
        image_object: FPDF_PAGEOBJECT,
        file_access: *mut FPDF_FILEACCESS,
    ) -> FPDF_BOOL {
        unsafe { (self.extern_FPDFImageObj_LoadJpegFile)(pages, count, image_object, file_access) }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FPDFImageObj_LoadJpegFileInline(
        &self,
        pages: *mut FPDF_PAGE,
        count: c_int,
        image_object: FPDF_PAGEOBJECT,
        file_access: *mut FPDF_FILEACCESS,
    ) -> FPDF_BOOL {
        unsafe { (self.extern_FPDFImageObj_LoadJpegFileInline)(pages, count, image_object, file_access) }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FPDFImageObj_SetMatrix(
        &self,
        image_object: FPDF_PAGEOBJECT,
        a: c_double,
        b: c_double,
        c: c_double,
        d: c_double,
        e: c_double,
        f: c_double,
    ) -> FPDF_BOOL {
        unsafe { (self.extern_FPDFImageObj_SetMatrix)(image_object, a, b, c, d, e, f) }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FPDFImageObj_SetBitmap(
        &self,
        pages: *mut FPDF_PAGE,
        count: c_int,
        image_object: FPDF_PAGEOBJECT,
        bitmap: FPDF_BITMAP,
    ) -> FPDF_BOOL {
        unsafe { (self.extern_FPDFImageObj_SetBitmap)(pages, count, image_object, bitmap) }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FPDFImageObj_GetBitmap(&self, image_object: FPDF_PAGEOBJECT) -> FPDF_BITMAP {
        unsafe { (self.extern_FPDFImageObj_GetBitmap)(image_object) }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FPDFImageObj_GetRenderedBitmap(
        &self,
        document: FPDF_DOCUMENT,
        page: FPDF_PAGE,
        image_object: FPDF_PAGEOBJECT,
    ) -> FPDF_BITMAP {
        unsafe { (self.extern_FPDFImageObj_GetRenderedBitmap)(document, page, image_object) }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FPDFImageObj_GetImageDataDecoded(
        &self,
        image_object: FPDF_PAGEOBJECT,
        buffer: *mut c_void,
        buflen: c_ulong,
    ) -> c_ulong {
        unsafe { (self.extern_FPDFImageObj_GetImageDataDecoded)(image_object, buffer, buflen) }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FPDFImageObj_GetImageDataRaw(
        &self,
        image_object: FPDF_PAGEOBJECT,
        buffer: *mut c_void,
        buflen: c_ulong,
    ) -> c_ulong {
        unsafe { (self.extern_FPDFImageObj_GetImageDataRaw)(image_object, buffer, buflen) }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FPDFImageObj_GetImageFilterCount(&self, image_object: FPDF_PAGEOBJECT) -> c_int {
        unsafe { (self.extern_FPDFImageObj_GetImageFilterCount)(image_object) }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FPDFImageObj_GetImageFilter(
        &self,
        image_object: FPDF_PAGEOBJECT,
        index: c_int,
        buffer: *mut c_void,
        buflen: c_ulong,
    ) -> c_ulong {
        unsafe { (self.extern_FPDFImageObj_GetImageFilter)(image_object, index, buffer, buflen) }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FPDFImageObj_GetImageMetadata(
        &self,
        image_object: FPDF_PAGEOBJECT,
        page: FPDF_PAGE,
        metadata: *mut FPDF_IMAGEOBJ_METADATA,
    ) -> FPDF_BOOL {
        unsafe { (self.extern_FPDFImageObj_GetImageMetadata)(image_object, page, metadata) }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FPDFImageObj_GetImagePixelSize(
        &self,
        image_object: FPDF_PAGEOBJECT,
        width: *mut c_uint,
        height: *mut c_uint,
    ) -> FPDF_BOOL {
        unsafe { (self.extern_FPDFImageObj_GetImagePixelSize)(image_object, width, height) }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FPDFImageObj_GetIccProfileDataDecoded(
        &self,
        image_object: FPDF_PAGEOBJECT,
        page: FPDF_PAGE,
        buffer: *mut u8,
        buflen: size_t,
        out_buflen: *mut size_t,
    ) -> FPDF_BOOL {
        unsafe { (self.extern_FPDFImageObj_GetIccProfileDataDecoded)(image_object, page, buffer, buflen, out_buflen) }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FPDFPageObj_CreateNewPath(&self, x: c_float, y: c_float) -> FPDF_PAGEOBJECT {
        unsafe { (self.extern_FPDFPageObj_CreateNewPath)(x, y) }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FPDFPageObj_CreateNewRect(&self, x: c_float, y: c_float, w: c_float, h: c_float) -> FPDF_PAGEOBJECT {
        unsafe { (self.extern_FPDFPageObj_CreateNewRect)(x, y, w, h) }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FPDFPageObj_GetBounds(
        &self,
        page_object: FPDF_PAGEOBJECT,
        left: *mut c_float,
        bottom: *mut c_float,
        right: *mut c_float,
        top: *mut c_float,
    ) -> FPDF_BOOL {
        unsafe { (self.extern_FPDFPageObj_GetBounds)(page_object, left, bottom, right, top) }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FPDFPageObj_GetRotatedBounds(
        &self,
        page_object: FPDF_PAGEOBJECT,
        quad_points: *mut FS_QUADPOINTSF,
    ) -> FPDF_BOOL {
        unsafe { (self.extern_FPDFPageObj_GetRotatedBounds)(page_object, quad_points) }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FPDFPageObj_SetBlendMode(&self, page_object: FPDF_PAGEOBJECT, blend_mode: &str) {
        let c_blend_mode = safe_cstring(blend_mode);

        unsafe { (self.extern_FPDFPageObj_SetBlendMode)(page_object, c_blend_mode.as_ptr()) }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FPDFPageObj_SetStrokeColor(
        &self,
        page_object: FPDF_PAGEOBJECT,
        R: c_uint,
        G: c_uint,
        B: c_uint,
        A: c_uint,
    ) -> FPDF_BOOL {
        unsafe { (self.extern_FPDFPageObj_SetStrokeColor)(page_object, R, G, B, A) }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FPDFPageObj_GetStrokeColor(
        &self,
        page_object: FPDF_PAGEOBJECT,
        R: *mut c_uint,
        G: *mut c_uint,
        B: *mut c_uint,
        A: *mut c_uint,
    ) -> FPDF_BOOL {
        unsafe { (self.extern_FPDFPageObj_GetStrokeColor)(page_object, R, G, B, A) }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FPDFPageObj_SetStrokeWidth(&self, page_object: FPDF_PAGEOBJECT, width: c_float) -> FPDF_BOOL {
        unsafe { (self.extern_FPDFPageObj_SetStrokeWidth)(page_object, width) }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FPDFPageObj_GetStrokeWidth(&self, page_object: FPDF_PAGEOBJECT, width: *mut c_float) -> FPDF_BOOL {
        unsafe { (self.extern_FPDFPageObj_GetStrokeWidth)(page_object, width) }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FPDFPageObj_GetLineJoin(&self, page_object: FPDF_PAGEOBJECT) -> c_int {
        unsafe { (self.extern_FPDFPageObj_GetLineJoin)(page_object) }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FPDFPageObj_SetLineJoin(&self, page_object: FPDF_PAGEOBJECT, line_join: c_int) -> FPDF_BOOL {
        unsafe { (self.extern_FPDFPageObj_SetLineJoin)(page_object, line_join) }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FPDFPageObj_GetLineCap(&self, page_object: FPDF_PAGEOBJECT) -> c_int {
        unsafe { (self.extern_FPDFPageObj_GetLineCap)(page_object) }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FPDFPageObj_SetLineCap(&self, page_object: FPDF_PAGEOBJECT, line_cap: c_int) -> FPDF_BOOL {
        unsafe { (self.extern_FPDFPageObj_SetLineCap)(page_object, line_cap) }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FPDFPageObj_SetFillColor(
        &self,
        page_object: FPDF_PAGEOBJECT,
        R: c_uint,
        G: c_uint,
        B: c_uint,
        A: c_uint,
    ) -> FPDF_BOOL {
        unsafe { (self.extern_FPDFPageObj_SetFillColor)(page_object, R, G, B, A) }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FPDFPageObj_GetFillColor(
        &self,
        page_object: FPDF_PAGEOBJECT,
        R: *mut c_uint,
        G: *mut c_uint,
        B: *mut c_uint,
        A: *mut c_uint,
    ) -> FPDF_BOOL {
        unsafe { (self.extern_FPDFPageObj_GetFillColor)(page_object, R, G, B, A) }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FPDFPageObj_GetDashPhase(&self, page_object: FPDF_PAGEOBJECT, phase: *mut c_float) -> FPDF_BOOL {
        unsafe { (self.extern_FPDFPageObj_GetDashPhase)(page_object, phase) }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FPDFPageObj_SetDashPhase(&self, page_object: FPDF_PAGEOBJECT, phase: c_float) -> FPDF_BOOL {
        unsafe { (self.extern_FPDFPageObj_SetDashPhase)(page_object, phase) }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FPDFPageObj_GetDashCount(&self, page_object: FPDF_PAGEOBJECT) -> c_int {
        unsafe { (self.extern_FPDFPageObj_GetDashCount)(page_object) }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FPDFPageObj_GetDashArray(
        &self,
        page_object: FPDF_PAGEOBJECT,
        dash_array: *mut c_float,
        dash_count: size_t,
    ) -> FPDF_BOOL {
        unsafe { (self.extern_FPDFPageObj_GetDashArray)(page_object, dash_array, dash_count) }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FPDFPageObj_SetDashArray(
        &self,
        page_object: FPDF_PAGEOBJECT,
        dash_array: *const c_float,
        dash_count: size_t,
        phase: c_float,
    ) -> FPDF_BOOL {
        unsafe { (self.extern_FPDFPageObj_SetDashArray)(page_object, dash_array, dash_count, phase) }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FPDFPath_CountSegments(&self, path: FPDF_PAGEOBJECT) -> c_int {
        unsafe { (self.extern_FPDFPath_CountSegments)(path) }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FPDFPath_GetPathSegment(&self, path: FPDF_PAGEOBJECT, index: c_int) -> FPDF_PATHSEGMENT {
        unsafe { (self.extern_FPDFPath_GetPathSegment)(path, index) }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FPDFPathSegment_GetPoint(&self, segment: FPDF_PATHSEGMENT, x: *mut c_float, y: *mut c_float) -> FPDF_BOOL {
        unsafe { (self.extern_FPDFPathSegment_GetPoint)(segment, x, y) }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FPDFPathSegment_GetType(&self, segment: FPDF_PATHSEGMENT) -> c_int {
        unsafe { (self.extern_FPDFPathSegment_GetType)(segment) }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FPDFPathSegment_GetClose(&self, segment: FPDF_PATHSEGMENT) -> FPDF_BOOL {
        unsafe { (self.extern_FPDFPathSegment_GetClose)(segment) }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FPDFFont_GetBaseFontName(&self, font: FPDF_FONT, buffer: *mut c_char, length: usize) -> usize {
        unsafe { (self.extern_FPDFFont_GetBaseFontName)(font, buffer, length) }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FPDFFont_GetFamilyName(&self, font: FPDF_FONT, buffer: *mut c_char, length: usize) -> usize {
        unsafe { (self.extern_FPDFFont_GetFamilyName)(font, buffer, length) }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FPDFFont_GetFontData(
        &self,
        font: FPDF_FONT,
        buffer: *mut u8,
        buflen: usize,
        out_buflen: *mut usize,
    ) -> FPDF_BOOL {
        unsafe { (self.extern_FPDFFont_GetFontData)(font, buffer, buflen, out_buflen) }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FPDFFont_GetIsEmbedded(&self, font: FPDF_FONT) -> c_int {
        unsafe { (self.extern_FPDFFont_GetIsEmbedded)(font) }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FPDFFont_GetFlags(&self, font: FPDF_FONT) -> c_int {
        unsafe { (self.extern_FPDFFont_GetFlags)(font) }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FPDFFont_GetWeight(&self, font: FPDF_FONT) -> c_int {
        unsafe { (self.extern_FPDFFont_GetWeight)(font) }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FPDFFont_GetItalicAngle(&self, font: FPDF_FONT, angle: *mut c_int) -> FPDF_BOOL {
        unsafe { (self.extern_FPDFFont_GetItalicAngle)(font, angle) }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FPDFFont_GetAscent(&self, font: FPDF_FONT, font_size: c_float, ascent: *mut c_float) -> FPDF_BOOL {
        unsafe { (self.extern_FPDFFont_GetAscent)(font, font_size, ascent) }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FPDFFont_GetDescent(&self, font: FPDF_FONT, font_size: c_float, descent: *mut c_float) -> FPDF_BOOL {
        unsafe { (self.extern_FPDFFont_GetDescent)(font, font_size, descent) }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FPDFFont_GetGlyphWidth(
        &self,
        font: FPDF_FONT,
        glyph: c_uint,
        font_size: c_float,
        width: *mut c_float,
    ) -> FPDF_BOOL {
        unsafe { (self.extern_FPDFFont_GetGlyphWidth)(font, glyph, font_size, width) }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FPDFFont_GetGlyphPath(&self, font: FPDF_FONT, glyph: c_uint, font_size: c_float) -> FPDF_GLYPHPATH {
        unsafe { (self.extern_FPDFFont_GetGlyphPath)(font, glyph, font_size) }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FPDFGlyphPath_CountGlyphSegments(&self, glyphpath: FPDF_GLYPHPATH) -> c_int {
        unsafe { (self.extern_FPDFGlyphPath_CountGlyphSegments)(glyphpath) }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FPDFGlyphPath_GetGlyphPathSegment(&self, glyphpath: FPDF_GLYPHPATH, index: c_int) -> FPDF_PATHSEGMENT {
        unsafe { (self.extern_FPDFGlyphPath_GetGlyphPathSegment)(glyphpath, index) }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FPDF_VIEWERREF_GetPrintScaling(&self, document: FPDF_DOCUMENT) -> FPDF_BOOL {
        unsafe { (self.extern_FPDF_VIEWERREF_GetPrintScaling)(document) }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FPDF_VIEWERREF_GetNumCopies(&self, document: FPDF_DOCUMENT) -> c_int {
        unsafe { (self.extern_FPDF_VIEWERREF_GetNumCopies)(document) }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FPDF_VIEWERREF_GetPrintPageRange(&self, document: FPDF_DOCUMENT) -> FPDF_PAGERANGE {
        unsafe { (self.extern_FPDF_VIEWERREF_GetPrintPageRange)(document) }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FPDF_VIEWERREF_GetPrintPageRangeCount(&self, pagerange: FPDF_PAGERANGE) -> size_t {
        unsafe { (self.extern_FPDF_VIEWERREF_GetPrintPageRangeCount)(pagerange) }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FPDF_VIEWERREF_GetPrintPageRangeElement(&self, pagerange: FPDF_PAGERANGE, index: size_t) -> c_int {
        unsafe { (self.extern_FPDF_VIEWERREF_GetPrintPageRangeElement)(pagerange, index) }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FPDF_VIEWERREF_GetDuplex(&self, document: FPDF_DOCUMENT) -> FPDF_DUPLEXTYPE {
        unsafe { (self.extern_FPDF_VIEWERREF_GetDuplex)(document) }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FPDF_VIEWERREF_GetName(
        &self,
        document: FPDF_DOCUMENT,
        key: &str,
        buffer: *mut c_char,
        length: c_ulong,
    ) -> c_ulong {
        let c_key = safe_cstring(key);

        unsafe { (self.extern_FPDF_VIEWERREF_GetName)(document, c_key.as_ptr(), buffer, length) }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FPDF_CountNamedDests(&self, document: FPDF_DOCUMENT) -> FPDF_DWORD {
        unsafe { (self.extern_FPDF_CountNamedDests)(document) }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FPDF_GetNamedDestByName(&self, document: FPDF_DOCUMENT, name: &str) -> FPDF_DEST {
        let c_name = safe_cstring(name);

        unsafe { (self.extern_FPDF_GetNamedDestByName)(document, c_name.as_ptr()) }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FPDF_GetNamedDest(
        &self,
        document: FPDF_DOCUMENT,
        index: c_int,
        buffer: *mut c_void,
        buflen: *mut c_long,
    ) -> FPDF_DEST {
        unsafe { (self.extern_FPDF_GetNamedDest)(document, index, buffer, buflen) }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FPDFDoc_GetAttachmentCount(&self, document: FPDF_DOCUMENT) -> c_int {
        unsafe { (self.extern_FPDFDoc_GetAttachmentCount)(document) }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FPDFDoc_AddAttachment(&self, document: FPDF_DOCUMENT, name: FPDF_WIDESTRING) -> FPDF_ATTACHMENT {
        unsafe { (self.extern_FPDFDoc_AddAttachment)(document, name) }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FPDFDoc_GetAttachment(&self, document: FPDF_DOCUMENT, index: c_int) -> FPDF_ATTACHMENT {
        unsafe { (self.extern_FPDFDoc_GetAttachment)(document, index) }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FPDFDoc_DeleteAttachment(&self, document: FPDF_DOCUMENT, index: c_int) -> FPDF_BOOL {
        unsafe { (self.extern_FPDFDoc_DeleteAttachment)(document, index) }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FPDFAttachment_GetName(&self, attachment: FPDF_ATTACHMENT, buffer: *mut FPDF_WCHAR, buflen: c_ulong) -> c_ulong {
        unsafe { (self.extern_FPDFAttachment_GetName)(attachment, buffer, buflen) }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FPDFAttachment_HasKey(&self, attachment: FPDF_ATTACHMENT, key: &str) -> FPDF_BOOL {
        let c_key = safe_cstring(key);

        unsafe { (self.extern_FPDFAttachment_HasKey)(attachment, c_key.as_ptr()) }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FPDFAttachment_GetValueType(&self, attachment: FPDF_ATTACHMENT, key: &str) -> FPDF_OBJECT_TYPE {
        let c_key = safe_cstring(key);

        unsafe { (self.extern_FPDFAttachment_GetValueType)(attachment, c_key.as_ptr()) }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FPDFAttachment_SetStringValue(
        &self,
        attachment: FPDF_ATTACHMENT,
        key: &str,
        value: FPDF_WIDESTRING,
    ) -> FPDF_BOOL {
        let c_key = safe_cstring(key);

        unsafe { (self.extern_FPDFAttachment_SetStringValue)(attachment, c_key.as_ptr(), value) }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FPDFAttachment_GetStringValue(
        &self,
        attachment: FPDF_ATTACHMENT,
        key: &str,
        buffer: *mut FPDF_WCHAR,
        buflen: c_ulong,
    ) -> c_ulong {
        let c_key = safe_cstring(key);

        unsafe { (self.extern_FPDFAttachment_GetStringValue)(attachment, c_key.as_ptr(), buffer, buflen) }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FPDFAttachment_SetFile(
        &self,
        attachment: FPDF_ATTACHMENT,
        document: FPDF_DOCUMENT,
        contents: *const c_void,
        len: c_ulong,
    ) -> FPDF_BOOL {
        unsafe { (self.extern_FPDFAttachment_SetFile)(attachment, document, contents, len) }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FPDFAttachment_GetFile(
        &self,
        attachment: FPDF_ATTACHMENT,
        buffer: *mut c_void,
        buflen: c_ulong,
        out_buflen: *mut c_ulong,
    ) -> FPDF_BOOL {
        unsafe { (self.extern_FPDFAttachment_GetFile)(attachment, buffer, buflen, out_buflen) }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FPDFAttachment_GetSubtype(
        &self,
        attachment: FPDF_ATTACHMENT,
        buffer: *mut FPDF_WCHAR,
        buflen: c_ulong,
    ) -> c_ulong {
        unsafe { (self.extern_FPDFAttachment_GetSubtype)(attachment, buffer, buflen) }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FPDFCatalog_IsTagged(&self, document: FPDF_DOCUMENT) -> FPDF_BOOL {
        unsafe { (self.extern_FPDFCatalog_IsTagged)(document) }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FPDFCatalog_SetLanguage(&self, document: FPDF_DOCUMENT, language: &str) -> FPDF_BOOL {
        let c_language = safe_cstring(language);

        unsafe { (self.extern_FPDFCatalog_SetLanguage)(document, c_language.as_ptr()) }
    }
}

impl Drop for DynamicPdfiumBindings {
    fn drop(&mut self) {
        self.FPDF_DestroyLibrary();
    }
}
