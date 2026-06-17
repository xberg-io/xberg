use crate::bindgen::{
    FPDF_ACTION, FPDF_ANNOT_APPEARANCEMODE, FPDF_ANNOTATION, FPDF_ANNOTATION_SUBTYPE, FPDF_ATTACHMENT, FPDF_AVAIL,
    FPDF_BITMAP, FPDF_BOOKMARK, FPDF_BOOL, FPDF_CLIPPATH, FPDF_COLORSCHEME, FPDF_CharsetFontMap, FPDF_DEST,
    FPDF_DOCUMENT, FPDF_DUPLEXTYPE, FPDF_DWORD, FPDF_FILEACCESS, FPDF_FILEIDTYPE, FPDF_FILEWRITE, FPDF_FONT,
    FPDF_FORMFILLINFO, FPDF_FORMHANDLE, FPDF_GLYPHPATH, FPDF_IMAGEOBJ_METADATA, FPDF_JAVASCRIPT_ACTION,
    FPDF_LIBRARY_CONFIG, FPDF_LINK, FPDF_OBJECT_TYPE, FPDF_PAGE, FPDF_PAGELINK, FPDF_PAGEOBJECT, FPDF_PAGEOBJECTMARK,
    FPDF_PAGERANGE, FPDF_PATHSEGMENT, FPDF_SCHHANDLE, FPDF_SIGNATURE, FPDF_STRUCTELEMENT, FPDF_STRUCTELEMENT_ATTR,
    FPDF_STRUCTTREE, FPDF_SYSFONTINFO, FPDF_TEXT_RENDERMODE, FPDF_TEXTPAGE, FPDF_WCHAR, FPDF_WIDESTRING, FPDF_XOBJECT,
    FPDFANNOT_COLORTYPE, FS_FLOAT, FS_MATRIX, FS_POINTF, FS_QUADPOINTSF, FS_RECTF, FS_SIZEF, FX_DOWNLOADHINTS,
    FX_FILEAVAIL, IFSDK_PAUSE, size_t,
};

use crate::bindgen::FPDF_STRUCTELEMENT_ATTR_VALUE;

use crate::bindings::PdfiumLibraryBindings;
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

pub(crate) struct StaticPdfiumBindings;

impl StaticPdfiumBindings {
    #[inline]
    pub fn new() -> Self {
        StaticPdfiumBindings {}
    }
}

impl Default for StaticPdfiumBindings {
    #[inline]
    fn default() -> Self {
        StaticPdfiumBindings::new()
    }
}

impl PdfiumLibraryBindings for StaticPdfiumBindings {
    #[inline]
    #[allow(non_snake_case)]
    fn FPDF_InitLibraryWithConfig(&self, config: *const FPDF_LIBRARY_CONFIG) {
        unsafe {
            crate::bindgen::FPDF_InitLibraryWithConfig(config);
        }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FPDF_InitLibrary(&self) {
        unsafe {
            crate::bindgen::FPDF_InitLibrary();
        }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FPDF_SetSandBoxPolicy(&self, policy: FPDF_DWORD, enable: FPDF_BOOL) {
        unsafe {
            crate::bindgen::FPDF_SetSandBoxPolicy(policy, enable);
        }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FPDF_DestroyLibrary(&self) {
        unsafe {
            crate::bindgen::FPDF_DestroyLibrary();
        }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FPDF_GetLastError(&self) -> c_ulong {
        unsafe { crate::bindgen::FPDF_GetLastError() }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FPDF_CreateNewDocument(&self) -> FPDF_DOCUMENT {
        unsafe { crate::bindgen::FPDF_CreateNewDocument() }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FPDF_LoadDocument(&self, file_path: &str, password: Option<&str>) -> FPDF_DOCUMENT {
        let c_file_path = safe_cstring(file_path);
        let c_password = safe_cstring(password.unwrap_or(""));

        unsafe { crate::bindgen::FPDF_LoadDocument(c_file_path.as_ptr(), c_password.as_ptr()) }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FPDF_LoadMemDocument64(&self, data_buf: &[u8], password: Option<&str>) -> FPDF_DOCUMENT {
        let c_password = safe_cstring(password.unwrap_or(""));

        unsafe {
            crate::bindgen::FPDF_LoadMemDocument64(
                data_buf.as_ptr() as *const c_void,
                data_buf.len() as size_t,
                c_password.as_ptr(),
            )
        }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FPDF_LoadCustomDocument(&self, pFileAccess: *mut FPDF_FILEACCESS, password: Option<&str>) -> FPDF_DOCUMENT {
        let c_password = safe_cstring(password.unwrap_or(""));

        unsafe { crate::bindgen::FPDF_LoadCustomDocument(pFileAccess, c_password.as_ptr()) }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FPDF_SaveAsCopy(
        &self,
        document: FPDF_DOCUMENT,
        pFileWrite: *mut FPDF_FILEWRITE,
        flags: FPDF_DWORD,
    ) -> FPDF_BOOL {
        unsafe { crate::bindgen::FPDF_SaveAsCopy(document, pFileWrite, flags) }
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
        unsafe { crate::bindgen::FPDF_SaveWithVersion(document, pFileWrite, flags, fileVersion) }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FPDFAvail_Create(&self, file_avail: *mut FX_FILEAVAIL, file: *mut FPDF_FILEACCESS) -> FPDF_AVAIL {
        unsafe { crate::bindgen::FPDFAvail_Create(file_avail, file) }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FPDFAvail_Destroy(&self, avail: FPDF_AVAIL) {
        unsafe { crate::bindgen::FPDFAvail_Destroy(avail) }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FPDFAvail_IsDocAvail(&self, avail: FPDF_AVAIL, hints: *mut FX_DOWNLOADHINTS) -> c_int {
        unsafe { crate::bindgen::FPDFAvail_IsDocAvail(avail, hints) }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FPDFAvail_GetDocument(&self, avail: FPDF_AVAIL, password: Option<&str>) -> FPDF_DOCUMENT {
        let c_password = safe_cstring(password.unwrap_or(""));

        unsafe { crate::bindgen::FPDFAvail_GetDocument(avail, c_password.as_ptr()) }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FPDFAvail_GetFirstPageNum(&self, doc: FPDF_DOCUMENT) -> c_int {
        unsafe { crate::bindgen::FPDFAvail_GetFirstPageNum(doc) }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FPDFAvail_IsPageAvail(&self, avail: FPDF_AVAIL, page_index: c_int, hints: *mut FX_DOWNLOADHINTS) -> c_int {
        unsafe { crate::bindgen::FPDFAvail_IsPageAvail(avail, page_index, hints) }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FPDFAvail_IsFormAvail(&self, avail: FPDF_AVAIL, hints: *mut FX_DOWNLOADHINTS) -> c_int {
        unsafe { crate::bindgen::FPDFAvail_IsFormAvail(avail, hints) }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FPDFAvail_IsLinearized(&self, avail: FPDF_AVAIL) -> c_int {
        unsafe { crate::bindgen::FPDFAvail_IsLinearized(avail) }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FPDF_CloseDocument(&self, document: FPDF_DOCUMENT) {
        unsafe {
            crate::bindgen::FPDF_CloseDocument(document);
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
            crate::bindgen::FPDF_DeviceToPage(
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
            crate::bindgen::FPDF_PageToDevice(
                page, start_x, start_y, size_x, size_y, rotate, page_x, page_y, device_x, device_y,
            )
        }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FPDF_GetFileVersion(&self, doc: FPDF_DOCUMENT, fileVersion: *mut c_int) -> FPDF_BOOL {
        unsafe { crate::bindgen::FPDF_GetFileVersion(doc, fileVersion) }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FPDF_DocumentHasValidCrossReferenceTable(&self, document: FPDF_DOCUMENT) -> FPDF_BOOL {
        unsafe { crate::bindgen::FPDF_DocumentHasValidCrossReferenceTable(document) }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FPDF_GetTrailerEnds(&self, document: FPDF_DOCUMENT, buffer: *mut c_uint, length: c_ulong) -> c_ulong {
        unsafe { crate::bindgen::FPDF_GetTrailerEnds(document, buffer, length) }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FPDF_GetDocPermissions(&self, document: FPDF_DOCUMENT) -> c_ulong {
        unsafe { crate::bindgen::FPDF_GetDocPermissions(document) }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FPDF_GetDocUserPermissions(&self, document: FPDF_DOCUMENT) -> c_ulong {
        unsafe { crate::bindgen::FPDF_GetDocUserPermissions(document) }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FPDF_GetSecurityHandlerRevision(&self, document: FPDF_DOCUMENT) -> c_int {
        unsafe { crate::bindgen::FPDF_GetSecurityHandlerRevision(document) }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FPDF_GetPageCount(&self, document: FPDF_DOCUMENT) -> c_int {
        unsafe { crate::bindgen::FPDF_GetPageCount(document) }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FPDF_LoadPage(&self, document: FPDF_DOCUMENT, page_index: c_int) -> FPDF_PAGE {
        unsafe { crate::bindgen::FPDF_LoadPage(document, page_index) }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FPDF_ClosePage(&self, page: FPDF_PAGE) {
        unsafe {
            crate::bindgen::FPDF_ClosePage(page);
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
            crate::bindgen::FPDF_RenderPageBitmapWithColorScheme_Start(
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
            crate::bindgen::FPDF_RenderPageBitmap_Start(
                bitmap, page, start_x, start_y, size_x, size_y, rotate, flags, pause,
            )
        }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FPDF_RenderPage_Continue(&self, page: FPDF_PAGE, pause: *mut IFSDK_PAUSE) -> c_int {
        unsafe { crate::bindgen::FPDF_RenderPage_Continue(page, pause) }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FPDF_RenderPage_Close(&self, page: FPDF_PAGE) {
        unsafe { crate::bindgen::FPDF_RenderPage_Close(page) }
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
        unsafe { crate::bindgen::FPDF_ImportPagesByIndex(dest_doc, src_doc, page_indices, length, index) }
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

        unsafe { crate::bindgen::FPDF_ImportPages(dest_doc, src_doc, c_pagerange.as_ptr(), index) }
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
            crate::bindgen::FPDF_ImportNPagesToOne(
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
        unsafe { crate::bindgen::FPDF_NewXObjectFromPage(dest_doc, src_doc, src_page_index) }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FPDF_CloseXObject(&self, xobject: FPDF_XOBJECT) {
        unsafe { crate::bindgen::FPDF_CloseXObject(xobject) }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FPDF_NewFormObjectFromXObject(&self, xobject: FPDF_XOBJECT) -> FPDF_PAGEOBJECT {
        unsafe { crate::bindgen::FPDF_NewFormObjectFromXObject(xobject) }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FPDF_CopyViewerPreferences(&self, dest_doc: FPDF_DOCUMENT, src_doc: FPDF_DOCUMENT) -> FPDF_BOOL {
        unsafe { crate::bindgen::FPDF_CopyViewerPreferences(dest_doc, src_doc) }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FPDF_GetPageWidthF(&self, page: FPDF_PAGE) -> c_float {
        unsafe { crate::bindgen::FPDF_GetPageWidthF(page) }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FPDF_GetPageWidth(&self, page: FPDF_PAGE) -> f64 {
        unsafe { crate::bindgen::FPDF_GetPageWidth(page) }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FPDF_GetPageHeightF(&self, page: FPDF_PAGE) -> c_float {
        unsafe { crate::bindgen::FPDF_GetPageHeightF(page) }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FPDF_GetPageHeight(&self, page: FPDF_PAGE) -> f64 {
        unsafe { crate::bindgen::FPDF_GetPageHeight(page) }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FPDFText_GetCharIndexFromTextIndex(&self, text_page: FPDF_TEXTPAGE, nTextIndex: c_int) -> c_int {
        unsafe { crate::bindgen::FPDFText_GetCharIndexFromTextIndex(text_page, nTextIndex) }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FPDFText_GetTextIndexFromCharIndex(&self, text_page: FPDF_TEXTPAGE, nCharIndex: c_int) -> c_int {
        unsafe { crate::bindgen::FPDFText_GetTextIndexFromCharIndex(text_page, nCharIndex) }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FPDF_GetSignatureCount(&self, document: FPDF_DOCUMENT) -> c_int {
        unsafe { crate::bindgen::FPDF_GetSignatureCount(document) }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FPDF_GetSignatureObject(&self, document: FPDF_DOCUMENT, index: c_int) -> FPDF_SIGNATURE {
        unsafe { crate::bindgen::FPDF_GetSignatureObject(document, index) }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FPDFSignatureObj_GetContents(&self, signature: FPDF_SIGNATURE, buffer: *mut c_void, length: c_ulong) -> c_ulong {
        unsafe { crate::bindgen::FPDFSignatureObj_GetContents(signature, buffer, length) }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FPDFSignatureObj_GetByteRange(&self, signature: FPDF_SIGNATURE, buffer: *mut c_int, length: c_ulong) -> c_ulong {
        unsafe { crate::bindgen::FPDFSignatureObj_GetByteRange(signature, buffer, length) }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FPDFSignatureObj_GetSubFilter(
        &self,
        signature: FPDF_SIGNATURE,
        buffer: *mut c_char,
        length: c_ulong,
    ) -> c_ulong {
        unsafe { crate::bindgen::FPDFSignatureObj_GetSubFilter(signature, buffer, length) }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FPDFSignatureObj_GetReason(&self, signature: FPDF_SIGNATURE, buffer: *mut c_void, length: c_ulong) -> c_ulong {
        unsafe { crate::bindgen::FPDFSignatureObj_GetReason(signature, buffer, length) }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FPDFSignatureObj_GetTime(&self, signature: FPDF_SIGNATURE, buffer: *mut c_char, length: c_ulong) -> c_ulong {
        unsafe { crate::bindgen::FPDFSignatureObj_GetTime(signature, buffer, length) }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FPDFSignatureObj_GetDocMDPPermission(&self, signature: FPDF_SIGNATURE) -> c_uint {
        unsafe { crate::bindgen::FPDFSignatureObj_GetDocMDPPermission(signature) }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FPDF_StructTree_GetForPage(&self, page: FPDF_PAGE) -> FPDF_STRUCTTREE {
        unsafe { crate::bindgen::FPDF_StructTree_GetForPage(page) }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FPDF_StructTree_Close(&self, struct_tree: FPDF_STRUCTTREE) {
        unsafe { crate::bindgen::FPDF_StructTree_Close(struct_tree) }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FPDF_StructTree_CountChildren(&self, struct_tree: FPDF_STRUCTTREE) -> c_int {
        unsafe { crate::bindgen::FPDF_StructTree_CountChildren(struct_tree) }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FPDF_StructTree_GetChildAtIndex(&self, struct_tree: FPDF_STRUCTTREE, index: c_int) -> FPDF_STRUCTELEMENT {
        unsafe { crate::bindgen::FPDF_StructTree_GetChildAtIndex(struct_tree, index) }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FPDF_StructElement_GetAltText(
        &self,
        struct_element: FPDF_STRUCTELEMENT,
        buffer: *mut c_void,
        buflen: c_ulong,
    ) -> c_ulong {
        unsafe { crate::bindgen::FPDF_StructElement_GetAltText(struct_element, buffer, buflen) }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FPDF_StructElement_GetActualText(
        &self,
        struct_element: FPDF_STRUCTELEMENT,
        buffer: *mut c_void,
        buflen: c_ulong,
    ) -> c_ulong {
        unsafe { crate::bindgen::FPDF_StructElement_GetActualText(struct_element, buffer, buflen) }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FPDF_StructElement_GetID(
        &self,
        struct_element: FPDF_STRUCTELEMENT,
        buffer: *mut c_void,
        buflen: c_ulong,
    ) -> c_ulong {
        unsafe { crate::bindgen::FPDF_StructElement_GetID(struct_element, buffer, buflen) }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FPDF_StructElement_GetLang(
        &self,
        struct_element: FPDF_STRUCTELEMENT,
        buffer: *mut c_void,
        buflen: c_ulong,
    ) -> c_ulong {
        unsafe { crate::bindgen::FPDF_StructElement_GetLang(struct_element, buffer, buflen) }
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
            crate::bindgen::FPDF_StructElement_GetStringAttribute(struct_element, c_attr_name.as_ptr(), buffer, buflen)
        }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FPDF_StructElement_GetMarkedContentID(&self, struct_element: FPDF_STRUCTELEMENT) -> c_int {
        unsafe { crate::bindgen::FPDF_StructElement_GetMarkedContentID(struct_element) }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FPDF_StructElement_GetType(
        &self,
        struct_element: FPDF_STRUCTELEMENT,
        buffer: *mut c_void,
        buflen: c_ulong,
    ) -> c_ulong {
        unsafe { crate::bindgen::FPDF_StructElement_GetType(struct_element, buffer, buflen) }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FPDF_StructElement_GetObjType(
        &self,
        struct_element: FPDF_STRUCTELEMENT,
        buffer: *mut c_void,
        buflen: c_ulong,
    ) -> c_ulong {
        unsafe { crate::bindgen::FPDF_StructElement_GetObjType(struct_element, buffer, buflen) }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FPDF_StructElement_GetTitle(
        &self,
        struct_element: FPDF_STRUCTELEMENT,
        buffer: *mut c_void,
        buflen: c_ulong,
    ) -> c_ulong {
        unsafe { crate::bindgen::FPDF_StructElement_GetTitle(struct_element, buffer, buflen) }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FPDF_StructElement_CountChildren(&self, struct_element: FPDF_STRUCTELEMENT) -> c_int {
        unsafe { crate::bindgen::FPDF_StructElement_CountChildren(struct_element) }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FPDF_StructElement_GetChildAtIndex(
        &self,
        struct_element: FPDF_STRUCTELEMENT,
        index: c_int,
    ) -> FPDF_STRUCTELEMENT {
        unsafe { crate::bindgen::FPDF_StructElement_GetChildAtIndex(struct_element, index) }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FPDF_StructElement_GetChildMarkedContentID(&self, struct_element: FPDF_STRUCTELEMENT, index: c_int) -> c_int {
        unsafe { crate::bindgen::FPDF_StructElement_GetChildMarkedContentID(struct_element, index) }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FPDF_StructElement_GetParent(&self, struct_element: FPDF_STRUCTELEMENT) -> FPDF_STRUCTELEMENT {
        unsafe { crate::bindgen::FPDF_StructElement_GetParent(struct_element) }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FPDF_StructElement_GetAttributeCount(&self, struct_element: FPDF_STRUCTELEMENT) -> c_int {
        unsafe { crate::bindgen::FPDF_StructElement_GetAttributeCount(struct_element) }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FPDF_StructElement_GetAttributeAtIndex(
        &self,
        struct_element: FPDF_STRUCTELEMENT,
        index: c_int,
    ) -> FPDF_STRUCTELEMENT_ATTR {
        unsafe { crate::bindgen::FPDF_StructElement_GetAttributeAtIndex(struct_element, index) }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FPDF_StructElement_Attr_GetCount(&self, struct_attribute: FPDF_STRUCTELEMENT_ATTR) -> c_int {
        unsafe { crate::bindgen::FPDF_StructElement_Attr_GetCount(struct_attribute) }
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
        unsafe { crate::bindgen::FPDF_StructElement_Attr_GetName(struct_attribute, index, buffer, buflen, out_buflen) }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FPDF_StructElement_Attr_GetValue(
        &self,
        struct_attribute: FPDF_STRUCTELEMENT_ATTR,
        name: &str,
    ) -> FPDF_STRUCTELEMENT_ATTR_VALUE {
        let c_name = safe_cstring(name);

        unsafe { crate::bindgen::FPDF_StructElement_Attr_GetValue(struct_attribute, c_name.as_ptr()) }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FPDF_StructElement_Attr_GetType(&self, value: FPDF_STRUCTELEMENT_ATTR_VALUE) -> FPDF_OBJECT_TYPE {
        unsafe { crate::bindgen::FPDF_StructElement_Attr_GetType(value) }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FPDF_StructElement_Attr_GetBooleanValue(
        &self,
        value: FPDF_STRUCTELEMENT_ATTR_VALUE,
        out_value: *mut FPDF_BOOL,
    ) -> FPDF_BOOL {
        unsafe { crate::bindgen::FPDF_StructElement_Attr_GetBooleanValue(value, out_value) }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FPDF_StructElement_Attr_GetNumberValue(
        &self,
        value: FPDF_STRUCTELEMENT_ATTR_VALUE,
        out_value: *mut f32,
    ) -> FPDF_BOOL {
        unsafe { crate::bindgen::FPDF_StructElement_Attr_GetNumberValue(value, out_value) }
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
        unsafe { crate::bindgen::FPDF_StructElement_Attr_GetStringValue(value, buffer, buflen, out_buflen) }
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
        unsafe { crate::bindgen::FPDF_StructElement_Attr_GetBlobValue(value, buffer, buflen, out_buflen) }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FPDF_StructElement_Attr_CountChildren(&self, value: FPDF_STRUCTELEMENT_ATTR_VALUE) -> c_int {
        unsafe { crate::bindgen::FPDF_StructElement_Attr_CountChildren(value) }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FPDF_StructElement_Attr_GetChildAtIndex(
        &self,
        value: FPDF_STRUCTELEMENT_ATTR_VALUE,
        index: c_int,
    ) -> FPDF_STRUCTELEMENT_ATTR_VALUE {
        unsafe { crate::bindgen::FPDF_StructElement_Attr_GetChildAtIndex(value, index) }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FPDF_StructElement_GetMarkedContentIdCount(&self, struct_element: FPDF_STRUCTELEMENT) -> c_int {
        unsafe { crate::bindgen::FPDF_StructElement_GetMarkedContentIdCount(struct_element) }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FPDF_StructElement_GetMarkedContentIdAtIndex(&self, struct_element: FPDF_STRUCTELEMENT, index: c_int) -> c_int {
        unsafe { crate::bindgen::FPDF_StructElement_GetMarkedContentIdAtIndex(struct_element, index) }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FPDFPage_New(&self, document: FPDF_DOCUMENT, page_index: c_int, width: c_double, height: c_double) -> FPDF_PAGE {
        unsafe { crate::bindgen::FPDFPage_New(document, page_index, width, height) }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FPDFPage_Delete(&self, document: FPDF_DOCUMENT, page_index: c_int) {
        unsafe { crate::bindgen::FPDFPage_Delete(document, page_index) }
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
        unsafe { crate::bindgen::FPDF_MovePages(document, page_indices, page_indices_len, dest_page_index) }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FPDFPage_GetRotation(&self, page: FPDF_PAGE) -> c_int {
        unsafe { crate::bindgen::FPDFPage_GetRotation(page) }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FPDFPage_SetRotation(&self, page: FPDF_PAGE, rotate: c_int) {
        unsafe { crate::bindgen::FPDFPage_SetRotation(page, rotate) }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FPDF_GetPageBoundingBox(&self, page: FPDF_PAGE, rect: *mut FS_RECTF) -> FPDF_BOOL {
        unsafe { crate::bindgen::FPDF_GetPageBoundingBox(page, rect) }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FPDF_GetPageSizeByIndexF(&self, document: FPDF_DOCUMENT, page_index: c_int, size: *mut FS_SIZEF) -> FPDF_BOOL {
        unsafe { crate::bindgen::FPDF_GetPageSizeByIndexF(document, page_index, size) }
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
        unsafe { crate::bindgen::FPDF_GetPageSizeByIndex(document, page_index, width, height) }
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
        unsafe { crate::bindgen::FPDFPage_GetMediaBox(page, left, bottom, right, top) }
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
        unsafe { crate::bindgen::FPDFPage_GetCropBox(page, left, bottom, right, top) }
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
        unsafe { crate::bindgen::FPDFPage_GetBleedBox(page, left, bottom, right, top) }
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
        unsafe { crate::bindgen::FPDFPage_GetTrimBox(page, left, bottom, right, top) }
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
        unsafe { crate::bindgen::FPDFPage_GetArtBox(page, left, bottom, right, top) }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FPDFPage_SetMediaBox(&self, page: FPDF_PAGE, left: c_float, bottom: c_float, right: c_float, top: c_float) {
        unsafe { crate::bindgen::FPDFPage_SetMediaBox(page, left, bottom, right, top) }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FPDFPage_SetCropBox(&self, page: FPDF_PAGE, left: c_float, bottom: c_float, right: c_float, top: c_float) {
        unsafe { crate::bindgen::FPDFPage_SetCropBox(page, left, bottom, right, top) }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FPDFPage_SetBleedBox(&self, page: FPDF_PAGE, left: c_float, bottom: c_float, right: c_float, top: c_float) {
        unsafe { crate::bindgen::FPDFPage_SetBleedBox(page, left, bottom, right, top) }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FPDFPage_SetTrimBox(&self, page: FPDF_PAGE, left: c_float, bottom: c_float, right: c_float, top: c_float) {
        unsafe { crate::bindgen::FPDFPage_SetTrimBox(page, left, bottom, right, top) }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FPDFPage_SetArtBox(&self, page: FPDF_PAGE, left: c_float, bottom: c_float, right: c_float, top: c_float) {
        unsafe { crate::bindgen::FPDFPage_SetArtBox(page, left, bottom, right, top) }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FPDFPage_TransFormWithClip(
        &self,
        page: FPDF_PAGE,
        matrix: *const FS_MATRIX,
        clipRect: *const FS_RECTF,
    ) -> FPDF_BOOL {
        unsafe { crate::bindgen::FPDFPage_TransFormWithClip(page, matrix, clipRect) }
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
        unsafe { crate::bindgen::FPDFPageObj_TransformClipPath(page_object, a, b, c, d, e, f) }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FPDFPageObj_GetClipPath(&self, page_object: FPDF_PAGEOBJECT) -> FPDF_CLIPPATH {
        unsafe { crate::bindgen::FPDFPageObj_GetClipPath(page_object) }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FPDFClipPath_CountPaths(&self, clip_path: FPDF_CLIPPATH) -> c_int {
        unsafe { crate::bindgen::FPDFClipPath_CountPaths(clip_path) }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FPDFClipPath_CountPathSegments(&self, clip_path: FPDF_CLIPPATH, path_index: c_int) -> c_int {
        unsafe { crate::bindgen::FPDFClipPath_CountPathSegments(clip_path, path_index) }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FPDFClipPath_GetPathSegment(
        &self,
        clip_path: FPDF_CLIPPATH,
        path_index: c_int,
        segment_index: c_int,
    ) -> FPDF_PATHSEGMENT {
        unsafe { crate::bindgen::FPDFClipPath_GetPathSegment(clip_path, path_index, segment_index) }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FPDF_CreateClipPath(&self, left: f32, bottom: f32, right: f32, top: f32) -> FPDF_CLIPPATH {
        unsafe { crate::bindgen::FPDF_CreateClipPath(left, bottom, right, top) }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FPDF_DestroyClipPath(&self, clipPath: FPDF_CLIPPATH) {
        unsafe { crate::bindgen::FPDF_DestroyClipPath(clipPath) }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FPDFPage_InsertClipPath(&self, page: FPDF_PAGE, clipPath: FPDF_CLIPPATH) {
        unsafe { crate::bindgen::FPDFPage_InsertClipPath(page, clipPath) }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FPDFPage_HasTransparency(&self, page: FPDF_PAGE) -> FPDF_BOOL {
        unsafe { crate::bindgen::FPDFPage_HasTransparency(page) }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FPDFPage_GenerateContent(&self, page: FPDF_PAGE) -> FPDF_BOOL {
        unsafe { crate::bindgen::FPDFPage_GenerateContent(page) }
    }

    #[allow(non_snake_case)]
    fn FPDFPage_TransformAnnots(&self, page: FPDF_PAGE, a: f64, b: f64, c: f64, d: f64, e: f64, f: f64) {
        unsafe { crate::bindgen::FPDFPage_TransformAnnots(page, a, b, c, d, e, f) }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FPDFBitmap_Create(&self, width: c_int, height: c_int, alpha: c_int) -> FPDF_BITMAP {
        unsafe { crate::bindgen::FPDFBitmap_Create(width, height, alpha) }
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
        unsafe { crate::bindgen::FPDFBitmap_CreateEx(width, height, format, first_scan, stride) }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FPDFBitmap_Destroy(&self, bitmap: FPDF_BITMAP) {
        unsafe { crate::bindgen::FPDFBitmap_Destroy(bitmap) }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FPDFBitmap_GetFormat(&self, bitmap: FPDF_BITMAP) -> c_int {
        unsafe { crate::bindgen::FPDFBitmap_GetFormat(bitmap) }
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
        unsafe { crate::bindgen::FPDFBitmap_FillRect(bitmap, left, top, width, height, color) }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FPDFBitmap_GetBuffer(&self, bitmap: FPDF_BITMAP) -> *mut c_void {
        unsafe { crate::bindgen::FPDFBitmap_GetBuffer(bitmap) }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FPDFBitmap_GetWidth(&self, bitmap: FPDF_BITMAP) -> c_int {
        unsafe { crate::bindgen::FPDFBitmap_GetWidth(bitmap) }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FPDFBitmap_GetHeight(&self, bitmap: FPDF_BITMAP) -> c_int {
        unsafe { crate::bindgen::FPDFBitmap_GetHeight(bitmap) }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FPDFBitmap_GetStride(&self, bitmap: FPDF_BITMAP) -> c_int {
        unsafe { crate::bindgen::FPDFBitmap_GetStride(bitmap) }
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
            crate::bindgen::FPDF_RenderPageBitmap(bitmap, page, start_x, start_y, size_x, size_y, rotate, flags);
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
            crate::bindgen::FPDF_RenderPageBitmapWithMatrix(bitmap, page, matrix, clipping, flags);
        }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FPDFAnnot_IsSupportedSubtype(&self, subtype: FPDF_ANNOTATION_SUBTYPE) -> FPDF_BOOL {
        unsafe { crate::bindgen::FPDFAnnot_IsSupportedSubtype(subtype) }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FPDFPage_CreateAnnot(&self, page: FPDF_PAGE, subtype: FPDF_ANNOTATION_SUBTYPE) -> FPDF_ANNOTATION {
        unsafe { crate::bindgen::FPDFPage_CreateAnnot(page, subtype) }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FPDFPage_GetAnnotCount(&self, page: FPDF_PAGE) -> c_int {
        unsafe { crate::bindgen::FPDFPage_GetAnnotCount(page) }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FPDFPage_GetAnnot(&self, page: FPDF_PAGE, index: c_int) -> FPDF_ANNOTATION {
        unsafe { crate::bindgen::FPDFPage_GetAnnot(page, index) }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FPDFPage_GetAnnotIndex(&self, page: FPDF_PAGE, annot: FPDF_ANNOTATION) -> c_int {
        unsafe { crate::bindgen::FPDFPage_GetAnnotIndex(page, annot) }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FPDFPage_CloseAnnot(&self, annot: FPDF_ANNOTATION) {
        unsafe { crate::bindgen::FPDFPage_CloseAnnot(annot) }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FPDFPage_RemoveAnnot(&self, page: FPDF_PAGE, index: c_int) -> FPDF_BOOL {
        unsafe { crate::bindgen::FPDFPage_RemoveAnnot(page, index) }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FPDFAnnot_GetSubtype(&self, annot: FPDF_ANNOTATION) -> FPDF_ANNOTATION_SUBTYPE {
        unsafe { crate::bindgen::FPDFAnnot_GetSubtype(annot) }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FPDFAnnot_IsObjectSupportedSubtype(&self, subtype: FPDF_ANNOTATION_SUBTYPE) -> FPDF_BOOL {
        unsafe { crate::bindgen::FPDFAnnot_IsObjectSupportedSubtype(subtype) }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FPDFAnnot_UpdateObject(&self, annot: FPDF_ANNOTATION, obj: FPDF_PAGEOBJECT) -> FPDF_BOOL {
        unsafe { crate::bindgen::FPDFAnnot_UpdateObject(annot, obj) }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FPDFAnnot_AddInkStroke(&self, annot: FPDF_ANNOTATION, points: *const FS_POINTF, point_count: size_t) -> c_int {
        unsafe { crate::bindgen::FPDFAnnot_AddInkStroke(annot, points, point_count) }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FPDFAnnot_RemoveInkList(&self, annot: FPDF_ANNOTATION) -> FPDF_BOOL {
        unsafe { crate::bindgen::FPDFAnnot_RemoveInkList(annot) }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FPDFAnnot_AppendObject(&self, annot: FPDF_ANNOTATION, obj: FPDF_PAGEOBJECT) -> FPDF_BOOL {
        unsafe { crate::bindgen::FPDFAnnot_AppendObject(annot, obj) }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FPDFAnnot_GetObjectCount(&self, annot: FPDF_ANNOTATION) -> c_int {
        unsafe { crate::bindgen::FPDFAnnot_GetObjectCount(annot) }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FPDFAnnot_GetObject(&self, annot: FPDF_ANNOTATION, index: c_int) -> FPDF_PAGEOBJECT {
        unsafe { crate::bindgen::FPDFAnnot_GetObject(annot, index) }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FPDFAnnot_RemoveObject(&self, annot: FPDF_ANNOTATION, index: c_int) -> FPDF_BOOL {
        unsafe { crate::bindgen::FPDFAnnot_RemoveObject(annot, index) }
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
        unsafe { crate::bindgen::FPDFAnnot_SetColor(annot, color_type, R, G, B, A) }
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
        unsafe { crate::bindgen::FPDFAnnot_GetColor(annot, color_type, R, G, B, A) }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FPDFAnnot_HasAttachmentPoints(&self, annot: FPDF_ANNOTATION) -> FPDF_BOOL {
        unsafe { crate::bindgen::FPDFAnnot_HasAttachmentPoints(annot) }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FPDFAnnot_SetAttachmentPoints(
        &self,
        annot: FPDF_ANNOTATION,
        quad_index: size_t,
        quad_points: *const FS_QUADPOINTSF,
    ) -> FPDF_BOOL {
        unsafe { crate::bindgen::FPDFAnnot_SetAttachmentPoints(annot, quad_index, quad_points) }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FPDFAnnot_AppendAttachmentPoints(
        &self,
        annot: FPDF_ANNOTATION,
        quad_points: *const FS_QUADPOINTSF,
    ) -> FPDF_BOOL {
        unsafe { crate::bindgen::FPDFAnnot_AppendAttachmentPoints(annot, quad_points) }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FPDFAnnot_CountAttachmentPoints(&self, annot: FPDF_ANNOTATION) -> size_t {
        unsafe { crate::bindgen::FPDFAnnot_CountAttachmentPoints(annot) }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FPDFAnnot_GetAttachmentPoints(
        &self,
        annot: FPDF_ANNOTATION,
        quad_index: size_t,
        quad_points: *mut FS_QUADPOINTSF,
    ) -> FPDF_BOOL {
        unsafe { crate::bindgen::FPDFAnnot_GetAttachmentPoints(annot, quad_index, quad_points) }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FPDFAnnot_SetRect(&self, annot: FPDF_ANNOTATION, rect: *const FS_RECTF) -> FPDF_BOOL {
        unsafe { crate::bindgen::FPDFAnnot_SetRect(annot, rect) }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FPDFAnnot_GetRect(&self, annot: FPDF_ANNOTATION, rect: *mut FS_RECTF) -> FPDF_BOOL {
        unsafe { crate::bindgen::FPDFAnnot_GetRect(annot, rect) }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FPDFAnnot_GetVertices(&self, annot: FPDF_ANNOTATION, buffer: *mut FS_POINTF, length: c_ulong) -> c_ulong {
        unsafe { crate::bindgen::FPDFAnnot_GetVertices(annot, buffer, length) }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FPDFAnnot_GetInkListCount(&self, annot: FPDF_ANNOTATION) -> c_ulong {
        unsafe { crate::bindgen::FPDFAnnot_GetInkListCount(annot) }
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
        unsafe { crate::bindgen::FPDFAnnot_GetInkListPath(annot, path_index, buffer, length) }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FPDFAnnot_GetLine(&self, annot: FPDF_ANNOTATION, start: *mut FS_POINTF, end: *mut FS_POINTF) -> FPDF_BOOL {
        unsafe { crate::bindgen::FPDFAnnot_GetLine(annot, start, end) }
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
        unsafe { crate::bindgen::FPDFAnnot_SetBorder(annot, horizontal_radius, vertical_radius, border_width) }
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
        unsafe { crate::bindgen::FPDFAnnot_GetBorder(annot, horizontal_radius, vertical_radius, border_width) }
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
        unsafe { crate::bindgen::FPDFAnnot_GetFormAdditionalActionJavaScript(form, annot, event, buffer, buflen) }
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
        unsafe { crate::bindgen::FPDFAnnot_GetFormFieldAlternateName(form, annot, buffer, buflen) }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FPDFAnnot_HasKey(&self, annot: FPDF_ANNOTATION, key: &str) -> FPDF_BOOL {
        let c_key = safe_cstring(key);

        unsafe { crate::bindgen::FPDFAnnot_HasKey(annot, c_key.as_ptr()) }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FPDFAnnot_GetValueType(&self, annot: FPDF_ANNOTATION, key: &str) -> FPDF_OBJECT_TYPE {
        let c_key = safe_cstring(key);

        unsafe { crate::bindgen::FPDFAnnot_GetValueType(annot, c_key.as_ptr()) }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FPDFAnnot_SetStringValue(&self, annot: FPDF_ANNOTATION, key: &str, value: FPDF_WIDESTRING) -> FPDF_BOOL {
        let c_key = safe_cstring(key);

        unsafe { crate::bindgen::FPDFAnnot_SetStringValue(annot, c_key.as_ptr(), value) }
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

        unsafe { crate::bindgen::FPDFAnnot_GetStringValue(annot, c_key.as_ptr(), buffer, buflen) }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FPDFAnnot_GetNumberValue(&self, annot: FPDF_ANNOTATION, key: &str, value: *mut f32) -> FPDF_BOOL {
        let c_key = safe_cstring(key);

        unsafe { crate::bindgen::FPDFAnnot_GetNumberValue(annot, c_key.as_ptr(), value) }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FPDFAnnot_SetAP(
        &self,
        annot: FPDF_ANNOTATION,
        appearanceMode: FPDF_ANNOT_APPEARANCEMODE,
        value: FPDF_WIDESTRING,
    ) -> FPDF_BOOL {
        unsafe { crate::bindgen::FPDFAnnot_SetAP(annot, appearanceMode, value) }
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
        unsafe { crate::bindgen::FPDFAnnot_GetAP(annot, appearanceMode, buffer, buflen) }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FPDFAnnot_GetLinkedAnnot(&self, annot: FPDF_ANNOTATION, key: &str) -> FPDF_ANNOTATION {
        let c_key = safe_cstring(key);

        unsafe { crate::bindgen::FPDFAnnot_GetLinkedAnnot(annot, c_key.as_ptr()) }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FPDFAnnot_GetFlags(&self, annot: FPDF_ANNOTATION) -> c_int {
        unsafe { crate::bindgen::FPDFAnnot_GetFlags(annot) }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FPDFAnnot_SetFlags(&self, annot: FPDF_ANNOTATION, flags: c_int) -> FPDF_BOOL {
        unsafe { crate::bindgen::FPDFAnnot_SetFlags(annot, flags) }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FPDFAnnot_GetFormFieldFlags(&self, form: FPDF_FORMHANDLE, annot: FPDF_ANNOTATION) -> c_int {
        unsafe { crate::bindgen::FPDFAnnot_GetFormFieldFlags(form, annot) }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FPDFAnnot_SetFormFieldFlags(&self, form: FPDF_FORMHANDLE, annot: FPDF_ANNOTATION, flags: c_int) -> FPDF_BOOL {
        unsafe { crate::bindgen::FPDFAnnot_SetFormFieldFlags(form, annot, flags) }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FPDFAnnot_GetFormFieldAtPoint(
        &self,
        form: FPDF_FORMHANDLE,
        page: FPDF_PAGE,
        point: *const FS_POINTF,
    ) -> FPDF_ANNOTATION {
        unsafe { crate::bindgen::FPDFAnnot_GetFormFieldAtPoint(form, page, point) }
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
        unsafe { crate::bindgen::FPDFAnnot_GetFormFieldName(form, annot, buffer, buflen) }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FPDFAnnot_GetFormFieldType(&self, form: FPDF_FORMHANDLE, annot: FPDF_ANNOTATION) -> c_int {
        unsafe { crate::bindgen::FPDFAnnot_GetFormFieldType(form, annot) }
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
        unsafe { crate::bindgen::FPDFAnnot_GetFormFieldValue(form, annot, buffer, buflen) }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FPDFAnnot_GetOptionCount(&self, form: FPDF_FORMHANDLE, annot: FPDF_ANNOTATION) -> c_int {
        unsafe { crate::bindgen::FPDFAnnot_GetOptionCount(form, annot) }
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
        unsafe { crate::bindgen::FPDFAnnot_GetOptionLabel(form, annot, index, buffer, buflen) }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FPDFAnnot_IsOptionSelected(&self, form: FPDF_FORMHANDLE, annot: FPDF_ANNOTATION, index: c_int) -> FPDF_BOOL {
        unsafe { crate::bindgen::FPDFAnnot_IsOptionSelected(form, annot, index) }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FPDFAnnot_GetFontSize(&self, form: FPDF_FORMHANDLE, annot: FPDF_ANNOTATION, value: *mut f32) -> FPDF_BOOL {
        unsafe { crate::bindgen::FPDFAnnot_GetFontSize(form, annot, value) }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FPDFAnnot_SetFontColor(
        &self,
        form: FPDF_FORMHANDLE,
        annot: FPDF_ANNOTATION,
        R: c_uint,
        G: c_uint,
        B: c_uint,
    ) -> FPDF_BOOL {
        unsafe { crate::bindgen::FPDFAnnot_SetFontColor(form, annot, R, G, B) }
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
        unsafe { crate::bindgen::FPDFAnnot_GetFontColor(form, annot, R, G, B) }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FPDFAnnot_IsChecked(&self, form: FPDF_FORMHANDLE, annot: FPDF_ANNOTATION) -> FPDF_BOOL {
        unsafe { crate::bindgen::FPDFAnnot_IsChecked(form, annot) }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FPDFAnnot_SetFocusableSubtypes(
        &self,
        form: FPDF_FORMHANDLE,
        subtypes: *const FPDF_ANNOTATION_SUBTYPE,
        count: size_t,
    ) -> FPDF_BOOL {
        unsafe { crate::bindgen::FPDFAnnot_SetFocusableSubtypes(form, subtypes, count) }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FPDFAnnot_GetFocusableSubtypesCount(&self, form: FPDF_FORMHANDLE) -> c_int {
        unsafe { crate::bindgen::FPDFAnnot_GetFocusableSubtypesCount(form) }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FPDFAnnot_GetFocusableSubtypes(
        &self,
        form: FPDF_FORMHANDLE,
        subtypes: *mut FPDF_ANNOTATION_SUBTYPE,
        count: size_t,
    ) -> FPDF_BOOL {
        unsafe { crate::bindgen::FPDFAnnot_GetFocusableSubtypes(form, subtypes, count) }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FPDFAnnot_GetLink(&self, annot: FPDF_ANNOTATION) -> FPDF_LINK {
        unsafe { crate::bindgen::FPDFAnnot_GetLink(annot) }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FPDFAnnot_GetFormControlCount(&self, form: FPDF_FORMHANDLE, annot: FPDF_ANNOTATION) -> c_int {
        unsafe { crate::bindgen::FPDFAnnot_GetFormControlCount(form, annot) }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FPDFAnnot_GetFormControlIndex(&self, form: FPDF_FORMHANDLE, annot: FPDF_ANNOTATION) -> c_int {
        unsafe { crate::bindgen::FPDFAnnot_GetFormControlIndex(form, annot) }
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
        unsafe { crate::bindgen::FPDFAnnot_GetFormFieldExportValue(form, annot, buffer, buflen) }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FPDFAnnot_SetURI(&self, annot: FPDF_ANNOTATION, uri: &str) -> FPDF_BOOL {
        let c_uri = safe_cstring(uri);

        unsafe { crate::bindgen::FPDFAnnot_SetURI(annot, c_uri.as_ptr()) }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FPDFAnnot_GetFileAttachment(&self, annot: FPDF_ANNOTATION) -> FPDF_ATTACHMENT {
        unsafe { crate::bindgen::FPDFAnnot_GetFileAttachment(annot) }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FPDFAnnot_AddFileAttachment(&self, annot: FPDF_ANNOTATION, name: FPDF_WIDESTRING) -> FPDF_ATTACHMENT {
        unsafe { crate::bindgen::FPDFAnnot_AddFileAttachment(annot, name) }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FPDFDOC_InitFormFillEnvironment(
        &self,
        document: FPDF_DOCUMENT,
        form_info: *mut FPDF_FORMFILLINFO,
    ) -> FPDF_FORMHANDLE {
        unsafe { crate::bindgen::FPDFDOC_InitFormFillEnvironment(document, form_info) }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FPDFDOC_ExitFormFillEnvironment(&self, form: FPDF_FORMHANDLE) {
        unsafe {
            crate::bindgen::FPDFDOC_ExitFormFillEnvironment(form);
        }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FORM_OnAfterLoadPage(&self, page: FPDF_PAGE, form: FPDF_FORMHANDLE) {
        unsafe { crate::bindgen::FORM_OnAfterLoadPage(page, form) }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FORM_OnBeforeClosePage(&self, page: FPDF_PAGE, form: FPDF_FORMHANDLE) {
        unsafe { crate::bindgen::FORM_OnBeforeClosePage(page, form) }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FPDFDoc_GetPageMode(&self, document: FPDF_DOCUMENT) -> c_int {
        unsafe { crate::bindgen::FPDFDoc_GetPageMode(document) }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FPDFPage_Flatten(&self, page: FPDF_PAGE, nFlag: c_int) -> c_int {
        unsafe { crate::bindgen::FPDFPage_Flatten(page, nFlag) }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FORM_DoDocumentJSAction(&self, form: FPDF_FORMHANDLE) {
        unsafe { crate::bindgen::FORM_DoDocumentJSAction(form) }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FORM_DoDocumentOpenAction(&self, form: FPDF_FORMHANDLE) {
        unsafe { crate::bindgen::FORM_DoDocumentOpenAction(form) }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FORM_DoDocumentAAction(&self, form: FPDF_FORMHANDLE, aaType: c_int) {
        unsafe { crate::bindgen::FORM_DoDocumentAAction(form, aaType) }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FORM_DoPageAAction(&self, page: FPDF_PAGE, form: FPDF_FORMHANDLE, aaType: c_int) {
        unsafe { crate::bindgen::FORM_DoPageAAction(page, form, aaType) }
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
        unsafe { crate::bindgen::FORM_OnMouseMove(form, page, modifier, page_x, page_y) }
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
        unsafe { crate::bindgen::FORM_OnMouseWheel(form, page, modifier, page_coord, delta_x, delta_y) }
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
        unsafe { crate::bindgen::FORM_OnFocus(form, page, modifier, page_x, page_y) }
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
        unsafe { crate::bindgen::FORM_OnLButtonDown(form, page, modifier, page_x, page_y) }
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
        unsafe { crate::bindgen::FORM_OnRButtonDown(form, page, modifier, page_x, page_y) }
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
        unsafe { crate::bindgen::FORM_OnLButtonUp(form, page, modifier, page_x, page_y) }
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
        unsafe { crate::bindgen::FORM_OnRButtonUp(form, page, modifier, page_x, page_y) }
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
        unsafe { crate::bindgen::FORM_OnLButtonDoubleClick(form, page, modifier, page_x, page_y) }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FORM_OnKeyDown(&self, form: FPDF_FORMHANDLE, page: FPDF_PAGE, nKeyCode: c_int, modifier: c_int) -> FPDF_BOOL {
        unsafe { crate::bindgen::FORM_OnKeyDown(form, page, nKeyCode, modifier) }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FORM_OnKeyUp(&self, form: FPDF_FORMHANDLE, page: FPDF_PAGE, nKeyCode: c_int, modifier: c_int) -> FPDF_BOOL {
        unsafe { crate::bindgen::FORM_OnKeyUp(form, page, nKeyCode, modifier) }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FORM_OnChar(&self, form: FPDF_FORMHANDLE, page: FPDF_PAGE, nChar: c_int, modifier: c_int) -> FPDF_BOOL {
        unsafe { crate::bindgen::FORM_OnChar(form, page, nChar, modifier) }
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
        unsafe { crate::bindgen::FORM_GetFocusedText(form, page, buffer, buflen) }
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
        unsafe { crate::bindgen::FORM_GetSelectedText(form, page, buffer, buflen) }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FORM_ReplaceAndKeepSelection(&self, form: FPDF_FORMHANDLE, page: FPDF_PAGE, wsText: FPDF_WIDESTRING) {
        unsafe { crate::bindgen::FORM_ReplaceAndKeepSelection(form, page, wsText) }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FORM_ReplaceSelection(&self, form: FPDF_FORMHANDLE, page: FPDF_PAGE, wsText: FPDF_WIDESTRING) {
        unsafe { crate::bindgen::FORM_ReplaceSelection(form, page, wsText) }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FORM_SelectAllText(&self, form: FPDF_FORMHANDLE, page: FPDF_PAGE) -> FPDF_BOOL {
        unsafe { crate::bindgen::FORM_SelectAllText(form, page) }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FORM_CanUndo(&self, form: FPDF_FORMHANDLE, page: FPDF_PAGE) -> FPDF_BOOL {
        unsafe { crate::bindgen::FORM_CanUndo(form, page) }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FORM_CanRedo(&self, form: FPDF_FORMHANDLE, page: FPDF_PAGE) -> FPDF_BOOL {
        unsafe { crate::bindgen::FORM_CanRedo(form, page) }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FORM_Undo(&self, form: FPDF_FORMHANDLE, page: FPDF_PAGE) -> FPDF_BOOL {
        unsafe { crate::bindgen::FORM_Undo(form, page) }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FORM_Redo(&self, form: FPDF_FORMHANDLE, page: FPDF_PAGE) -> FPDF_BOOL {
        unsafe { crate::bindgen::FORM_Redo(form, page) }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FORM_ForceToKillFocus(&self, form: FPDF_FORMHANDLE) -> FPDF_BOOL {
        unsafe { crate::bindgen::FORM_ForceToKillFocus(form) }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FORM_GetFocusedAnnot(
        &self,
        form: FPDF_FORMHANDLE,
        page_index: *mut c_int,
        annot: *mut FPDF_ANNOTATION,
    ) -> FPDF_BOOL {
        unsafe { crate::bindgen::FORM_GetFocusedAnnot(form, page_index, annot) }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FORM_SetFocusedAnnot(&self, form: FPDF_FORMHANDLE, annot: FPDF_ANNOTATION) -> FPDF_BOOL {
        unsafe { crate::bindgen::FORM_SetFocusedAnnot(form, annot) }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FPDFPage_HasFormFieldAtPoint(&self, form: FPDF_FORMHANDLE, page: FPDF_PAGE, page_x: f64, page_y: f64) -> c_int {
        unsafe { crate::bindgen::FPDFPage_HasFormFieldAtPoint(form, page, page_x, page_y) }
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
        unsafe { crate::bindgen::FPDFPage_FormFieldZOrderAtPoint(form, page, page_x, page_y) }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FPDF_SetFormFieldHighlightColor(&self, form: FPDF_FORMHANDLE, field_type: c_int, color: FPDF_DWORD) {
        unsafe {
            crate::bindgen::FPDF_SetFormFieldHighlightColor(form, field_type, color);
        }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FPDF_SetFormFieldHighlightAlpha(&self, form: FPDF_FORMHANDLE, alpha: c_uchar) {
        unsafe {
            crate::bindgen::FPDF_SetFormFieldHighlightAlpha(form, alpha);
        }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FPDF_RemoveFormFieldHighlight(&self, form: FPDF_FORMHANDLE) {
        unsafe { crate::bindgen::FPDF_RemoveFormFieldHighlight(form) }
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
            crate::bindgen::FPDF_FFLDraw(form, bitmap, page, start_x, start_y, size_x, size_y, rotate, flags);
        }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FPDF_GetFormType(&self, document: FPDF_DOCUMENT) -> c_int {
        unsafe { crate::bindgen::FPDF_GetFormType(document) }
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
        unsafe { crate::bindgen::FORM_SetIndexSelected(form, page, index, selected) }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FORM_IsIndexSelected(&self, form: FPDF_FORMHANDLE, page: FPDF_PAGE, index: c_int) -> FPDF_BOOL {
        unsafe { crate::bindgen::FORM_IsIndexSelected(form, page, index) }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FPDF_LoadXFA(&self, document: FPDF_DOCUMENT) -> FPDF_BOOL {
        unsafe { crate::bindgen::FPDF_LoadXFA(document) }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FPDFDoc_GetJavaScriptActionCount(&self, document: FPDF_DOCUMENT) -> c_int {
        unsafe { crate::bindgen::FPDFDoc_GetJavaScriptActionCount(document) }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FPDFDoc_GetJavaScriptAction(&self, document: FPDF_DOCUMENT, index: c_int) -> FPDF_JAVASCRIPT_ACTION {
        unsafe { crate::bindgen::FPDFDoc_GetJavaScriptAction(document, index) }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FPDFDoc_CloseJavaScriptAction(&self, javascript: FPDF_JAVASCRIPT_ACTION) {
        unsafe { crate::bindgen::FPDFDoc_CloseJavaScriptAction(javascript) }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FPDFJavaScriptAction_GetName(
        &self,
        javascript: FPDF_JAVASCRIPT_ACTION,
        buffer: *mut FPDF_WCHAR,
        buflen: c_ulong,
    ) -> c_ulong {
        unsafe { crate::bindgen::FPDFJavaScriptAction_GetName(javascript, buffer, buflen) }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FPDFJavaScriptAction_GetScript(
        &self,
        javascript: FPDF_JAVASCRIPT_ACTION,
        buffer: *mut FPDF_WCHAR,
        buflen: c_ulong,
    ) -> c_ulong {
        unsafe { crate::bindgen::FPDFJavaScriptAction_GetScript(javascript, buffer, buflen) }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FPDF_GetDefaultTTFMap(&self) -> *const FPDF_CharsetFontMap {
        unsafe { crate::bindgen::FPDF_GetDefaultTTFMap() }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FPDF_GetDefaultTTFMapCount(&self) -> usize {
        unsafe { crate::bindgen::FPDF_GetDefaultTTFMapCount() }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FPDF_GetDefaultTTFMapEntry(&self, index: usize) -> *const FPDF_CharsetFontMap {
        unsafe { crate::bindgen::FPDF_GetDefaultTTFMapEntry(index) }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FPDF_AddInstalledFont(&self, mapper: *mut c_void, face: &str, charset: c_int) {
        let c_face = safe_cstring(face);

        unsafe { crate::bindgen::FPDF_AddInstalledFont(mapper, c_face.as_ptr(), charset) }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FPDF_SetSystemFontInfo(&self, pFontInfo: *mut FPDF_SYSFONTINFO) {
        unsafe { crate::bindgen::FPDF_SetSystemFontInfo(pFontInfo) }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FPDF_GetDefaultSystemFontInfo(&self) -> *mut FPDF_SYSFONTINFO {
        unsafe { crate::bindgen::FPDF_GetDefaultSystemFontInfo() }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FPDF_FreeDefaultSystemFontInfo(&self, pFontInfo: *mut FPDF_SYSFONTINFO) {
        unsafe { crate::bindgen::FPDF_FreeDefaultSystemFontInfo(pFontInfo) }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FPDFBookmark_GetFirstChild(&self, document: FPDF_DOCUMENT, bookmark: FPDF_BOOKMARK) -> FPDF_BOOKMARK {
        unsafe { crate::bindgen::FPDFBookmark_GetFirstChild(document, bookmark) }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FPDFBookmark_GetNextSibling(&self, document: FPDF_DOCUMENT, bookmark: FPDF_BOOKMARK) -> FPDF_BOOKMARK {
        unsafe { crate::bindgen::FPDFBookmark_GetNextSibling(document, bookmark) }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FPDFBookmark_GetTitle(&self, bookmark: FPDF_BOOKMARK, buffer: *mut c_void, buflen: c_ulong) -> c_ulong {
        unsafe { crate::bindgen::FPDFBookmark_GetTitle(bookmark, buffer, buflen) }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FPDFBookmark_GetCount(&self, bookmark: FPDF_BOOKMARK) -> c_int {
        unsafe { crate::bindgen::FPDFBookmark_GetCount(bookmark) }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FPDFBookmark_Find(&self, document: FPDF_DOCUMENT, title: FPDF_WIDESTRING) -> FPDF_BOOKMARK {
        unsafe { crate::bindgen::FPDFBookmark_Find(document, title) }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FPDFBookmark_GetDest(&self, document: FPDF_DOCUMENT, bookmark: FPDF_BOOKMARK) -> FPDF_DEST {
        unsafe { crate::bindgen::FPDFBookmark_GetDest(document, bookmark) }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FPDFBookmark_GetAction(&self, bookmark: FPDF_BOOKMARK) -> FPDF_ACTION {
        unsafe { crate::bindgen::FPDFBookmark_GetAction(bookmark) }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FPDFAction_GetType(&self, action: FPDF_ACTION) -> c_ulong {
        unsafe { crate::bindgen::FPDFAction_GetType(action) }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FPDFAction_GetDest(&self, document: FPDF_DOCUMENT, action: FPDF_ACTION) -> FPDF_DEST {
        unsafe { crate::bindgen::FPDFAction_GetDest(document, action) }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FPDFAction_GetFilePath(&self, action: FPDF_ACTION, buffer: *mut c_void, buflen: c_ulong) -> c_ulong {
        unsafe { crate::bindgen::FPDFAction_GetFilePath(action, buffer, buflen) }
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
        unsafe { crate::bindgen::FPDFAction_GetURIPath(document, action, buffer, buflen) }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FPDFDest_GetDestPageIndex(&self, document: FPDF_DOCUMENT, dest: FPDF_DEST) -> c_int {
        unsafe { crate::bindgen::FPDFDest_GetDestPageIndex(document, dest) }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FPDFDest_GetView(&self, dest: FPDF_DEST, pNumParams: *mut c_ulong, pParams: *mut FS_FLOAT) -> c_ulong {
        unsafe { crate::bindgen::FPDFDest_GetView(dest, pNumParams, pParams) }
    }

    #[inline]
    #[allow(non_snake_case)]
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
        unsafe { crate::bindgen::FPDFDest_GetLocationInPage(dest, hasXVal, hasYVal, hasZoomVal, x, y, zoom) }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FPDFLink_GetLinkAtPoint(&self, page: FPDF_PAGE, x: c_double, y: c_double) -> FPDF_LINK {
        unsafe { crate::bindgen::FPDFLink_GetLinkAtPoint(page, x, y) }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FPDFLink_GetLinkZOrderAtPoint(&self, page: FPDF_PAGE, x: c_double, y: c_double) -> c_int {
        unsafe { crate::bindgen::FPDFLink_GetLinkZOrderAtPoint(page, x, y) }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FPDFLink_GetDest(&self, document: FPDF_DOCUMENT, link: FPDF_LINK) -> FPDF_DEST {
        unsafe { crate::bindgen::FPDFLink_GetDest(document, link) }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FPDFLink_GetAction(&self, link: FPDF_LINK) -> FPDF_ACTION {
        unsafe { crate::bindgen::FPDFLink_GetAction(link) }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FPDFLink_Enumerate(&self, page: FPDF_PAGE, start_pos: *mut c_int, link_annot: *mut FPDF_LINK) -> FPDF_BOOL {
        unsafe { crate::bindgen::FPDFLink_Enumerate(page, start_pos, link_annot) }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FPDFLink_GetAnnot(&self, page: FPDF_PAGE, link_annot: FPDF_LINK) -> FPDF_ANNOTATION {
        unsafe { crate::bindgen::FPDFLink_GetAnnot(page, link_annot) }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FPDFLink_GetAnnotRect(&self, link_annot: FPDF_LINK, rect: *mut FS_RECTF) -> FPDF_BOOL {
        unsafe { crate::bindgen::FPDFLink_GetAnnotRect(link_annot, rect) }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FPDFLink_CountQuadPoints(&self, link_annot: FPDF_LINK) -> c_int {
        unsafe { crate::bindgen::FPDFLink_CountQuadPoints(link_annot) }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FPDFLink_GetQuadPoints(
        &self,
        link_annot: FPDF_LINK,
        quad_index: c_int,
        quad_points: *mut FS_QUADPOINTSF,
    ) -> FPDF_BOOL {
        unsafe { crate::bindgen::FPDFLink_GetQuadPoints(link_annot, quad_index, quad_points) }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FPDF_GetPageAAction(&self, page: FPDF_PAGE, aa_type: c_int) -> FPDF_ACTION {
        unsafe { crate::bindgen::FPDF_GetPageAAction(page, aa_type) }
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
        unsafe { crate::bindgen::FPDF_GetFileIdentifier(document, id_type, buffer, buflen) }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FPDF_GetMetaText(&self, document: FPDF_DOCUMENT, tag: &str, buffer: *mut c_void, buflen: c_ulong) -> c_ulong {
        let c_tag = safe_cstring(tag);

        unsafe { crate::bindgen::FPDF_GetMetaText(document, c_tag.as_ptr(), buffer, buflen) }
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
        unsafe { crate::bindgen::FPDF_GetPageLabel(document, page_index, buffer, buflen) }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FPDFText_LoadPage(&self, page: FPDF_PAGE) -> FPDF_TEXTPAGE {
        unsafe { crate::bindgen::FPDFText_LoadPage(page) }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FPDFText_ClosePage(&self, text_page: FPDF_TEXTPAGE) {
        unsafe {
            crate::bindgen::FPDFText_ClosePage(text_page);
        }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FPDFText_CountChars(&self, text_page: FPDF_TEXTPAGE) -> c_int {
        unsafe { crate::bindgen::FPDFText_CountChars(text_page) }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FPDFText_GetUnicode(&self, text_page: FPDF_TEXTPAGE, index: c_int) -> c_uint {
        unsafe { crate::bindgen::FPDFText_GetUnicode(text_page, index) }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FPDFText_GetTextObject(&self, text_page: FPDF_TEXTPAGE, index: c_int) -> FPDF_PAGEOBJECT {
        unsafe { crate::bindgen::FPDFText_GetTextObject(text_page, index) }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FPDFText_IsGenerated(&self, text_page: FPDF_TEXTPAGE, index: c_int) -> c_int {
        unsafe { crate::bindgen::FPDFText_IsGenerated(text_page, index) }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FPDFText_IsHyphen(&self, text_page: FPDF_TEXTPAGE, index: c_int) -> c_int {
        unsafe { crate::bindgen::FPDFText_IsHyphen(text_page, index) }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FPDFText_HasUnicodeMapError(&self, text_page: FPDF_TEXTPAGE, index: c_int) -> c_int {
        unsafe { crate::bindgen::FPDFText_HasUnicodeMapError(text_page, index) }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FPDFText_GetFontSize(&self, text_page: FPDF_TEXTPAGE, index: c_int) -> c_double {
        unsafe { crate::bindgen::FPDFText_GetFontSize(text_page, index) }
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
        unsafe { crate::bindgen::FPDFText_GetFontInfo(text_page, index, buffer, buflen, flags) }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FPDFText_GetFontWeight(&self, text_page: FPDF_TEXTPAGE, index: c_int) -> c_int {
        unsafe { crate::bindgen::FPDFText_GetFontWeight(text_page, index) }
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
        unsafe { crate::bindgen::FPDFText_GetFillColor(text_page, index, R, G, B, A) }
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
        unsafe { crate::bindgen::FPDFText_GetStrokeColor(text_page, index, R, G, B, A) }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FPDFText_GetCharAngle(&self, text_page: FPDF_TEXTPAGE, index: c_int) -> c_float {
        unsafe { crate::bindgen::FPDFText_GetCharAngle(text_page, index) }
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
        unsafe { crate::bindgen::FPDFText_GetCharBox(text_page, index, left, right, bottom, top) }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FPDFText_GetLooseCharBox(&self, text_page: FPDF_TEXTPAGE, index: c_int, rect: *mut FS_RECTF) -> FPDF_BOOL {
        unsafe { crate::bindgen::FPDFText_GetLooseCharBox(text_page, index, rect) }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FPDFText_GetMatrix(&self, text_page: FPDF_TEXTPAGE, index: c_int, matrix: *mut FS_MATRIX) -> FPDF_BOOL {
        unsafe { crate::bindgen::FPDFText_GetMatrix(text_page, index, matrix) }
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
        unsafe { crate::bindgen::FPDFText_GetCharOrigin(text_page, index, x, y) }
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
        unsafe { crate::bindgen::FPDFText_GetCharIndexAtPos(text_page, x, y, xTolerance, yTolerance) }
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
        unsafe { crate::bindgen::FPDFText_GetText(text_page, start_index, count, result) }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FPDFText_CountRects(&self, text_page: FPDF_TEXTPAGE, start_index: c_int, count: c_int) -> c_int {
        unsafe { crate::bindgen::FPDFText_CountRects(text_page, start_index, count) }
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
        unsafe { crate::bindgen::FPDFText_GetRect(text_page, rect_index, left, top, right, bottom) }
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
        unsafe { crate::bindgen::FPDFText_GetBoundedText(text_page, left, top, right, bottom, buffer, buflen) }
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
        unsafe { crate::bindgen::FPDFText_FindStart(text_page, findwhat, flags, start_index) }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FPDFText_FindNext(&self, handle: FPDF_SCHHANDLE) -> FPDF_BOOL {
        unsafe { crate::bindgen::FPDFText_FindNext(handle) }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FPDFText_FindPrev(&self, handle: FPDF_SCHHANDLE) -> FPDF_BOOL {
        unsafe { crate::bindgen::FPDFText_FindPrev(handle) }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FPDFText_GetSchResultIndex(&self, handle: FPDF_SCHHANDLE) -> c_int {
        unsafe { crate::bindgen::FPDFText_GetSchResultIndex(handle) }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FPDFText_GetSchCount(&self, handle: FPDF_SCHHANDLE) -> c_int {
        unsafe { crate::bindgen::FPDFText_GetSchCount(handle) }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FPDFText_FindClose(&self, handle: FPDF_SCHHANDLE) {
        unsafe { crate::bindgen::FPDFText_FindClose(handle) }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FPDFLink_LoadWebLinks(&self, text_page: FPDF_TEXTPAGE) -> FPDF_PAGELINK {
        unsafe { crate::bindgen::FPDFLink_LoadWebLinks(text_page) }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FPDFLink_CountWebLinks(&self, link_page: FPDF_PAGELINK) -> c_int {
        unsafe { crate::bindgen::FPDFLink_CountWebLinks(link_page) }
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
        unsafe { crate::bindgen::FPDFLink_GetURL(link_page, link_index, buffer, buflen) }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FPDFLink_CountRects(&self, link_page: FPDF_PAGELINK, link_index: c_int) -> c_int {
        unsafe { crate::bindgen::FPDFLink_CountRects(link_page, link_index) }
    }

    #[inline]
    #[allow(non_snake_case)]
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
        unsafe { crate::bindgen::FPDFLink_GetRect(link_page, link_index, rect_index, left, top, right, bottom) }
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
        unsafe { crate::bindgen::FPDFLink_GetTextRange(link_page, link_index, start_char_index, char_count) }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FPDFLink_CloseWebLinks(&self, link_page: FPDF_PAGELINK) {
        unsafe { crate::bindgen::FPDFLink_CloseWebLinks(link_page) }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FPDFPage_GetDecodedThumbnailData(&self, page: FPDF_PAGE, buffer: *mut c_void, buflen: c_ulong) -> c_ulong {
        unsafe { crate::bindgen::FPDFPage_GetDecodedThumbnailData(page, buffer, buflen) }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FPDFPage_GetRawThumbnailData(&self, page: FPDF_PAGE, buffer: *mut c_void, buflen: c_ulong) -> c_ulong {
        unsafe { crate::bindgen::FPDFPage_GetRawThumbnailData(page, buffer, buflen) }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FPDFPage_GetThumbnailAsBitmap(&self, page: FPDF_PAGE) -> FPDF_BITMAP {
        unsafe { crate::bindgen::FPDFPage_GetThumbnailAsBitmap(page) }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FPDFFormObj_CountObjects(&self, form_object: FPDF_PAGEOBJECT) -> c_int {
        unsafe { crate::bindgen::FPDFFormObj_CountObjects(form_object) }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FPDFFormObj_GetObject(&self, form_object: FPDF_PAGEOBJECT, index: c_ulong) -> FPDF_PAGEOBJECT {
        unsafe { crate::bindgen::FPDFFormObj_GetObject(form_object, index) }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FPDFFormObj_RemoveObject(&self, form_object: FPDF_PAGEOBJECT, page_object: FPDF_PAGEOBJECT) -> FPDF_BOOL {
        unsafe { crate::bindgen::FPDFFormObj_RemoveObject(form_object, page_object) }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FPDFPageObj_CreateTextObj(
        &self,
        document: FPDF_DOCUMENT,
        font: FPDF_FONT,
        font_size: c_float,
    ) -> FPDF_PAGEOBJECT {
        unsafe { crate::bindgen::FPDFPageObj_CreateTextObj(document, font, font_size) }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FPDFTextObj_GetTextRenderMode(&self, text: FPDF_PAGEOBJECT) -> FPDF_TEXT_RENDERMODE {
        unsafe { crate::bindgen::FPDFTextObj_GetTextRenderMode(text) }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FPDFTextObj_SetTextRenderMode(&self, text: FPDF_PAGEOBJECT, render_mode: FPDF_TEXT_RENDERMODE) -> FPDF_BOOL {
        unsafe { crate::bindgen::FPDFTextObj_SetTextRenderMode(text, render_mode) }
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
        unsafe { crate::bindgen::FPDFTextObj_GetText(text_object, text_page, buffer, length) }
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
        unsafe { crate::bindgen::FPDFTextObj_GetRenderedBitmap(document, page, text_object, scale) }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FPDFTextObj_GetFont(&self, text: FPDF_PAGEOBJECT) -> FPDF_FONT {
        unsafe { crate::bindgen::FPDFTextObj_GetFont(text) }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FPDFTextObj_GetFontSize(&self, text: FPDF_PAGEOBJECT, size: *mut c_float) -> FPDF_BOOL {
        unsafe { crate::bindgen::FPDFTextObj_GetFontSize(text, size) }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FPDFFont_Close(&self, font: FPDF_FONT) {
        unsafe { crate::bindgen::FPDFFont_Close(font) }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FPDFPath_MoveTo(&self, path: FPDF_PAGEOBJECT, x: c_float, y: c_float) -> FPDF_BOOL {
        unsafe { crate::bindgen::FPDFPath_MoveTo(path, x, y) }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FPDFPath_LineTo(&self, path: FPDF_PAGEOBJECT, x: c_float, y: c_float) -> FPDF_BOOL {
        unsafe { crate::bindgen::FPDFPath_LineTo(path, x, y) }
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
        unsafe { crate::bindgen::FPDFPath_BezierTo(path, x1, y1, x2, y2, x3, y3) }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FPDFPath_Close(&self, path: FPDF_PAGEOBJECT) -> FPDF_BOOL {
        unsafe { crate::bindgen::FPDFPath_Close(path) }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FPDFPath_SetDrawMode(&self, path: FPDF_PAGEOBJECT, fillmode: c_int, stroke: FPDF_BOOL) -> FPDF_BOOL {
        unsafe { crate::bindgen::FPDFPath_SetDrawMode(path, fillmode, stroke) }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FPDFPath_GetDrawMode(&self, path: FPDF_PAGEOBJECT, fillmode: *mut c_int, stroke: *mut FPDF_BOOL) -> FPDF_BOOL {
        unsafe { crate::bindgen::FPDFPath_GetDrawMode(path, fillmode, stroke) }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FPDFPageObj_NewTextObj(&self, document: FPDF_DOCUMENT, font: &str, font_size: c_float) -> FPDF_PAGEOBJECT {
        let c_font = safe_cstring(font);

        unsafe { crate::bindgen::FPDFPageObj_NewTextObj(document, c_font.as_ptr(), font_size) }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FPDFText_SetText(&self, text_object: FPDF_PAGEOBJECT, text: FPDF_WIDESTRING) -> FPDF_BOOL {
        unsafe { crate::bindgen::FPDFText_SetText(text_object, text) }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FPDFText_SetCharcodes(
        &self,
        text_object: FPDF_PAGEOBJECT,
        charcodes: *const c_uint,
        count: size_t,
    ) -> FPDF_BOOL {
        unsafe { crate::bindgen::FPDFText_SetCharcodes(text_object, charcodes, count) }
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
        unsafe { crate::bindgen::FPDFText_LoadFont(document, data, size, font_type, cid) }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FPDFText_LoadStandardFont(&self, document: FPDF_DOCUMENT, font: &str) -> FPDF_FONT {
        let c_font = safe_cstring(font);

        unsafe { crate::bindgen::FPDFText_LoadStandardFont(document, c_font.as_ptr()) }
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
            crate::bindgen::FPDFText_LoadCidType2Font(
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
    fn FPDFPage_InsertObject(&self, page: FPDF_PAGE, page_obj: FPDF_PAGEOBJECT) {
        unsafe { crate::bindgen::FPDFPage_InsertObject(page, page_obj) }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FPDFPage_InsertObjectAtIndex(&self, page: FPDF_PAGE, page_object: FPDF_PAGEOBJECT, index: usize) -> FPDF_BOOL {
        unsafe { crate::bindgen::FPDFPage_InsertObjectAtIndex(page, page_object, index) }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FPDFPage_RemoveObject(&self, page: FPDF_PAGE, page_obj: FPDF_PAGEOBJECT) -> FPDF_BOOL {
        unsafe { crate::bindgen::FPDFPage_RemoveObject(page, page_obj) }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FPDFPage_CountObjects(&self, page: FPDF_PAGE) -> c_int {
        unsafe { crate::bindgen::FPDFPage_CountObjects(page) }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FPDFPage_GetObject(&self, page: FPDF_PAGE, index: c_int) -> FPDF_PAGEOBJECT {
        unsafe { crate::bindgen::FPDFPage_GetObject(page, index) }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FPDFPageObj_Destroy(&self, page_obj: FPDF_PAGEOBJECT) {
        unsafe { crate::bindgen::FPDFPageObj_Destroy(page_obj) }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FPDFPageObj_HasTransparency(&self, page_object: FPDF_PAGEOBJECT) -> FPDF_BOOL {
        unsafe { crate::bindgen::FPDFPageObj_HasTransparency(page_object) }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FPDFPageObj_GetType(&self, page_object: FPDF_PAGEOBJECT) -> c_int {
        unsafe { crate::bindgen::FPDFPageObj_GetType(page_object) }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FPDFPageObj_GetIsActive(&self, page_object: FPDF_PAGEOBJECT, active: *mut FPDF_BOOL) -> FPDF_BOOL {
        unsafe { crate::bindgen::FPDFPageObj_GetIsActive(page_object, active) }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FPDFPageObj_SetIsActive(&self, page_object: FPDF_PAGEOBJECT, active: FPDF_BOOL) -> FPDF_BOOL {
        unsafe { crate::bindgen::FPDFPageObj_SetIsActive(page_object, active) }
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
        unsafe { crate::bindgen::FPDFPageObj_Transform(page_object, a, b, c, d, e, f) }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FPDFPageObj_TransformF(&self, page_object: FPDF_PAGEOBJECT, matrix: *const FS_MATRIX) -> FPDF_BOOL {
        unsafe { crate::bindgen::FPDFPageObj_TransformF(page_object, matrix) }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FPDFPageObj_GetMatrix(&self, page_object: FPDF_PAGEOBJECT, matrix: *mut FS_MATRIX) -> FPDF_BOOL {
        unsafe { crate::bindgen::FPDFPageObj_GetMatrix(page_object, matrix) }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FPDFPageObj_SetMatrix(&self, path: FPDF_PAGEOBJECT, matrix: *const FS_MATRIX) -> FPDF_BOOL {
        unsafe { crate::bindgen::FPDFPageObj_SetMatrix(path, matrix) }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FPDFPageObj_NewImageObj(&self, document: FPDF_DOCUMENT) -> FPDF_PAGEOBJECT {
        unsafe { crate::bindgen::FPDFPageObj_NewImageObj(document) }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FPDFPageObj_GetMarkedContentID(&self, page_object: FPDF_PAGEOBJECT) -> c_int {
        unsafe { crate::bindgen::FPDFPageObj_GetMarkedContentID(page_object) }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FPDFPageObj_CountMarks(&self, page_object: FPDF_PAGEOBJECT) -> c_int {
        unsafe { crate::bindgen::FPDFPageObj_CountMarks(page_object) }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FPDFPageObj_GetMark(&self, page_object: FPDF_PAGEOBJECT, index: c_ulong) -> FPDF_PAGEOBJECTMARK {
        unsafe { crate::bindgen::FPDFPageObj_GetMark(page_object, index) }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FPDFPageObj_AddMark(&self, page_object: FPDF_PAGEOBJECT, name: &str) -> FPDF_PAGEOBJECTMARK {
        let c_name = safe_cstring(name);

        unsafe { crate::bindgen::FPDFPageObj_AddMark(page_object, c_name.as_ptr()) }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FPDFPageObj_RemoveMark(&self, page_object: FPDF_PAGEOBJECT, mark: FPDF_PAGEOBJECTMARK) -> FPDF_BOOL {
        unsafe { crate::bindgen::FPDFPageObj_RemoveMark(page_object, mark) }
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
        unsafe { crate::bindgen::FPDFPageObjMark_GetName(mark, buffer, buflen, out_buflen) }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FPDFPageObjMark_CountParams(&self, mark: FPDF_PAGEOBJECTMARK) -> c_int {
        unsafe { crate::bindgen::FPDFPageObjMark_CountParams(mark) }
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
        unsafe { crate::bindgen::FPDFPageObjMark_GetParamKey(mark, index, buffer, buflen, out_buflen) }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FPDFPageObjMark_GetParamValueType(&self, mark: FPDF_PAGEOBJECTMARK, key: &str) -> FPDF_OBJECT_TYPE {
        let c_key = safe_cstring(key);

        unsafe { crate::bindgen::FPDFPageObjMark_GetParamValueType(mark, c_key.as_ptr()) }
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

        unsafe { crate::bindgen::FPDFPageObjMark_GetParamIntValue(mark, c_key.as_ptr(), out_value) }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FPDFPageObjMark_GetParamFloatValue(
        &self,
        mark: FPDF_PAGEOBJECTMARK,
        key: &str,
        out_value: *mut c_float,
    ) -> FPDF_BOOL {
        let c_key = safe_cstring(key);

        unsafe { crate::bindgen::FPDFPageObjMark_GetParamFloatValue(mark, c_key.as_ptr(), out_value) }
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

        unsafe { crate::bindgen::FPDFPageObjMark_GetParamStringValue(mark, c_key.as_ptr(), buffer, buflen, out_buflen) }
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

        unsafe { crate::bindgen::FPDFPageObjMark_GetParamBlobValue(mark, c_key.as_ptr(), buffer, buflen, out_buflen) }
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

        unsafe { crate::bindgen::FPDFPageObjMark_SetIntParam(document, page_object, mark, c_key.as_ptr(), value) }
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

        unsafe { crate::bindgen::FPDFPageObjMark_SetFloatParam(document, page_object, mark, c_key.as_ptr(), value) }
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
            crate::bindgen::FPDFPageObjMark_SetStringParam(
                document,
                page_object,
                mark,
                c_key.as_ptr(),
                c_value.as_ptr(),
            )
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
            crate::bindgen::FPDFPageObjMark_SetBlobParam(document, page_object, mark, c_key.as_ptr(), value, value_len)
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

        unsafe { crate::bindgen::FPDFPageObjMark_RemoveParam(page_object, mark, c_key.as_ptr()) }
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
        unsafe { crate::bindgen::FPDFImageObj_LoadJpegFile(pages, count, image_object, file_access) }
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
        unsafe { crate::bindgen::FPDFImageObj_LoadJpegFileInline(pages, count, image_object, file_access) }
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
        unsafe { crate::bindgen::FPDFImageObj_SetMatrix(image_object, a, b, c, d, e, f) }
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
        unsafe { crate::bindgen::FPDFImageObj_SetBitmap(pages, count, image_object, bitmap) }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FPDFImageObj_GetBitmap(&self, image_object: FPDF_PAGEOBJECT) -> FPDF_BITMAP {
        unsafe { crate::bindgen::FPDFImageObj_GetBitmap(image_object) }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FPDFImageObj_GetRenderedBitmap(
        &self,
        document: FPDF_DOCUMENT,
        page: FPDF_PAGE,
        image_object: FPDF_PAGEOBJECT,
    ) -> FPDF_BITMAP {
        unsafe { crate::bindgen::FPDFImageObj_GetRenderedBitmap(document, page, image_object) }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FPDFImageObj_GetImageDataDecoded(
        &self,
        image_object: FPDF_PAGEOBJECT,
        buffer: *mut c_void,
        buflen: c_ulong,
    ) -> c_ulong {
        unsafe { crate::bindgen::FPDFImageObj_GetImageDataDecoded(image_object, buffer, buflen) }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FPDFImageObj_GetImageDataRaw(
        &self,
        image_object: FPDF_PAGEOBJECT,
        buffer: *mut c_void,
        buflen: c_ulong,
    ) -> c_ulong {
        unsafe { crate::bindgen::FPDFImageObj_GetImageDataRaw(image_object, buffer, buflen) }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FPDFImageObj_GetImageFilterCount(&self, image_object: FPDF_PAGEOBJECT) -> c_int {
        unsafe { crate::bindgen::FPDFImageObj_GetImageFilterCount(image_object) }
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
        unsafe { crate::bindgen::FPDFImageObj_GetImageFilter(image_object, index, buffer, buflen) }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FPDFImageObj_GetImageMetadata(
        &self,
        image_object: FPDF_PAGEOBJECT,
        page: FPDF_PAGE,
        metadata: *mut FPDF_IMAGEOBJ_METADATA,
    ) -> FPDF_BOOL {
        unsafe { crate::bindgen::FPDFImageObj_GetImageMetadata(image_object, page, metadata) }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FPDFImageObj_GetImagePixelSize(
        &self,
        image_object: FPDF_PAGEOBJECT,
        width: *mut c_uint,
        height: *mut c_uint,
    ) -> FPDF_BOOL {
        unsafe { crate::bindgen::FPDFImageObj_GetImagePixelSize(image_object, width, height) }
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
        unsafe { crate::bindgen::FPDFImageObj_GetIccProfileDataDecoded(image_object, page, buffer, buflen, out_buflen) }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FPDFPageObj_CreateNewPath(&self, x: c_float, y: c_float) -> FPDF_PAGEOBJECT {
        unsafe { crate::bindgen::FPDFPageObj_CreateNewPath(x, y) }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FPDFPageObj_CreateNewRect(&self, x: c_float, y: c_float, w: c_float, h: c_float) -> FPDF_PAGEOBJECT {
        unsafe { crate::bindgen::FPDFPageObj_CreateNewRect(x, y, w, h) }
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
        unsafe { crate::bindgen::FPDFPageObj_GetBounds(page_object, left, bottom, right, top) }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FPDFPageObj_GetRotatedBounds(
        &self,
        page_object: FPDF_PAGEOBJECT,
        quad_points: *mut FS_QUADPOINTSF,
    ) -> FPDF_BOOL {
        unsafe { crate::bindgen::FPDFPageObj_GetRotatedBounds(page_object, quad_points) }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FPDFPageObj_SetBlendMode(&self, page_object: FPDF_PAGEOBJECT, blend_mode: &str) {
        let c_blend_mode = safe_cstring(blend_mode);

        unsafe { crate::bindgen::FPDFPageObj_SetBlendMode(page_object, c_blend_mode.as_ptr()) }
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
        unsafe { crate::bindgen::FPDFPageObj_SetStrokeColor(page_object, R, G, B, A) }
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
        unsafe { crate::bindgen::FPDFPageObj_GetStrokeColor(page_object, R, G, B, A) }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FPDFPageObj_SetStrokeWidth(&self, page_object: FPDF_PAGEOBJECT, width: c_float) -> FPDF_BOOL {
        unsafe { crate::bindgen::FPDFPageObj_SetStrokeWidth(page_object, width) }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FPDFPageObj_GetStrokeWidth(&self, page_object: FPDF_PAGEOBJECT, width: *mut c_float) -> FPDF_BOOL {
        unsafe { crate::bindgen::FPDFPageObj_GetStrokeWidth(page_object, width) }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FPDFPageObj_GetLineJoin(&self, page_object: FPDF_PAGEOBJECT) -> c_int {
        unsafe { crate::bindgen::FPDFPageObj_GetLineJoin(page_object) }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FPDFPageObj_SetLineJoin(&self, page_object: FPDF_PAGEOBJECT, line_join: c_int) -> FPDF_BOOL {
        unsafe { crate::bindgen::FPDFPageObj_SetLineJoin(page_object, line_join) }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FPDFPageObj_GetLineCap(&self, page_object: FPDF_PAGEOBJECT) -> c_int {
        unsafe { crate::bindgen::FPDFPageObj_GetLineCap(page_object) }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FPDFPageObj_SetLineCap(&self, page_object: FPDF_PAGEOBJECT, line_cap: c_int) -> FPDF_BOOL {
        unsafe { crate::bindgen::FPDFPageObj_SetLineCap(page_object, line_cap) }
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
        unsafe { crate::bindgen::FPDFPageObj_SetFillColor(page_object, R, G, B, A) }
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
        unsafe { crate::bindgen::FPDFPageObj_GetFillColor(page_object, R, G, B, A) }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FPDFPageObj_GetDashPhase(&self, page_object: FPDF_PAGEOBJECT, phase: *mut c_float) -> FPDF_BOOL {
        unsafe { crate::bindgen::FPDFPageObj_GetDashPhase(page_object, phase) }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FPDFPageObj_SetDashPhase(&self, page_object: FPDF_PAGEOBJECT, phase: c_float) -> FPDF_BOOL {
        unsafe { crate::bindgen::FPDFPageObj_SetDashPhase(page_object, phase) }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FPDFPageObj_GetDashCount(&self, page_object: FPDF_PAGEOBJECT) -> c_int {
        unsafe { crate::bindgen::FPDFPageObj_GetDashCount(page_object) }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FPDFPageObj_GetDashArray(
        &self,
        page_object: FPDF_PAGEOBJECT,
        dash_array: *mut c_float,
        dash_count: size_t,
    ) -> FPDF_BOOL {
        unsafe { crate::bindgen::FPDFPageObj_GetDashArray(page_object, dash_array, dash_count) }
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
        unsafe { crate::bindgen::FPDFPageObj_SetDashArray(page_object, dash_array, dash_count, phase) }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FPDFPath_CountSegments(&self, path: FPDF_PAGEOBJECT) -> c_int {
        unsafe { crate::bindgen::FPDFPath_CountSegments(path) }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FPDFPath_GetPathSegment(&self, path: FPDF_PAGEOBJECT, index: c_int) -> FPDF_PATHSEGMENT {
        unsafe { crate::bindgen::FPDFPath_GetPathSegment(path, index) }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FPDFPathSegment_GetPoint(&self, segment: FPDF_PATHSEGMENT, x: *mut c_float, y: *mut c_float) -> FPDF_BOOL {
        unsafe { crate::bindgen::FPDFPathSegment_GetPoint(segment, x, y) }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FPDFPathSegment_GetType(&self, segment: FPDF_PATHSEGMENT) -> c_int {
        unsafe { crate::bindgen::FPDFPathSegment_GetType(segment) }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FPDFPathSegment_GetClose(&self, segment: FPDF_PATHSEGMENT) -> FPDF_BOOL {
        unsafe { crate::bindgen::FPDFPathSegment_GetClose(segment) }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FPDFFont_GetBaseFontName(&self, font: FPDF_FONT, buffer: *mut c_char, length: usize) -> usize {
        unsafe { crate::bindgen::FPDFFont_GetBaseFontName(font, buffer, length) }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FPDFFont_GetFamilyName(
        &self,
        font: FPDF_FONT,
        buffer: *mut c_char,
        length: usize, // size_t is used in Pdfium API header, so usize is appropriate here
    ) -> usize // size_t is used in Pdfium API header, so usize is appropriate here
    {
        unsafe { crate::bindgen::FPDFFont_GetBaseFontName(font, buffer, length) }
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
        unsafe { crate::bindgen::FPDFFont_GetFontData(font, buffer, buflen, out_buflen) }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FPDFFont_GetIsEmbedded(&self, font: FPDF_FONT) -> c_int {
        unsafe { crate::bindgen::FPDFFont_GetIsEmbedded(font) }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FPDFFont_GetFlags(&self, font: FPDF_FONT) -> c_int {
        unsafe { crate::bindgen::FPDFFont_GetFlags(font) }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FPDFFont_GetWeight(&self, font: FPDF_FONT) -> c_int {
        unsafe { crate::bindgen::FPDFFont_GetWeight(font) }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FPDFFont_GetItalicAngle(&self, font: FPDF_FONT, angle: *mut c_int) -> FPDF_BOOL {
        unsafe { crate::bindgen::FPDFFont_GetItalicAngle(font, angle) }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FPDFFont_GetAscent(&self, font: FPDF_FONT, font_size: c_float, ascent: *mut c_float) -> FPDF_BOOL {
        unsafe { crate::bindgen::FPDFFont_GetAscent(font, font_size, ascent) }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FPDFFont_GetDescent(&self, font: FPDF_FONT, font_size: c_float, descent: *mut c_float) -> FPDF_BOOL {
        unsafe { crate::bindgen::FPDFFont_GetDescent(font, font_size, descent) }
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
        unsafe { crate::bindgen::FPDFFont_GetGlyphWidth(font, glyph, font_size, width) }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FPDFFont_GetGlyphPath(&self, font: FPDF_FONT, glyph: c_uint, font_size: c_float) -> FPDF_GLYPHPATH {
        unsafe { crate::bindgen::FPDFFont_GetGlyphPath(font, glyph, font_size) }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FPDFGlyphPath_CountGlyphSegments(&self, glyphpath: FPDF_GLYPHPATH) -> c_int {
        unsafe { crate::bindgen::FPDFGlyphPath_CountGlyphSegments(glyphpath) }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FPDFGlyphPath_GetGlyphPathSegment(&self, glyphpath: FPDF_GLYPHPATH, index: c_int) -> FPDF_PATHSEGMENT {
        unsafe { crate::bindgen::FPDFGlyphPath_GetGlyphPathSegment(glyphpath, index) }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FPDF_VIEWERREF_GetPrintScaling(&self, document: FPDF_DOCUMENT) -> FPDF_BOOL {
        unsafe { crate::bindgen::FPDF_VIEWERREF_GetPrintScaling(document) }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FPDF_VIEWERREF_GetNumCopies(&self, document: FPDF_DOCUMENT) -> c_int {
        unsafe { crate::bindgen::FPDF_VIEWERREF_GetNumCopies(document) }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FPDF_VIEWERREF_GetPrintPageRange(&self, document: FPDF_DOCUMENT) -> FPDF_PAGERANGE {
        unsafe { crate::bindgen::FPDF_VIEWERREF_GetPrintPageRange(document) }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FPDF_VIEWERREF_GetPrintPageRangeCount(&self, pagerange: FPDF_PAGERANGE) -> size_t {
        unsafe { crate::bindgen::FPDF_VIEWERREF_GetPrintPageRangeCount(pagerange) }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FPDF_VIEWERREF_GetPrintPageRangeElement(&self, pagerange: FPDF_PAGERANGE, index: size_t) -> c_int {
        unsafe { crate::bindgen::FPDF_VIEWERREF_GetPrintPageRangeElement(pagerange, index) }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FPDF_VIEWERREF_GetDuplex(&self, document: FPDF_DOCUMENT) -> FPDF_DUPLEXTYPE {
        unsafe { crate::bindgen::FPDF_VIEWERREF_GetDuplex(document) }
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

        unsafe { crate::bindgen::FPDF_VIEWERREF_GetName(document, c_key.as_ptr(), buffer, length) }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FPDF_CountNamedDests(&self, document: FPDF_DOCUMENT) -> FPDF_DWORD {
        unsafe { crate::bindgen::FPDF_CountNamedDests(document) }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FPDF_GetNamedDestByName(&self, document: FPDF_DOCUMENT, name: &str) -> FPDF_DEST {
        let c_name = safe_cstring(name);

        unsafe { crate::bindgen::FPDF_GetNamedDestByName(document, c_name.as_ptr()) }
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
        unsafe { crate::bindgen::FPDF_GetNamedDest(document, index, buffer, buflen) }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FPDFDoc_GetAttachmentCount(&self, document: FPDF_DOCUMENT) -> c_int {
        unsafe { crate::bindgen::FPDFDoc_GetAttachmentCount(document) }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FPDFDoc_AddAttachment(&self, document: FPDF_DOCUMENT, name: FPDF_WIDESTRING) -> FPDF_ATTACHMENT {
        unsafe { crate::bindgen::FPDFDoc_AddAttachment(document, name) }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FPDFDoc_GetAttachment(&self, document: FPDF_DOCUMENT, index: c_int) -> FPDF_ATTACHMENT {
        unsafe { crate::bindgen::FPDFDoc_GetAttachment(document, index) }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FPDFDoc_DeleteAttachment(&self, document: FPDF_DOCUMENT, index: c_int) -> FPDF_BOOL {
        unsafe { crate::bindgen::FPDFDoc_DeleteAttachment(document, index) }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FPDFAttachment_GetName(&self, attachment: FPDF_ATTACHMENT, buffer: *mut FPDF_WCHAR, buflen: c_ulong) -> c_ulong {
        unsafe { crate::bindgen::FPDFAttachment_GetName(attachment, buffer, buflen) }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FPDFAttachment_HasKey(&self, attachment: FPDF_ATTACHMENT, key: &str) -> FPDF_BOOL {
        let c_key = safe_cstring(key);

        unsafe { crate::bindgen::FPDFAttachment_HasKey(attachment, c_key.as_ptr()) }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FPDFAttachment_GetValueType(&self, attachment: FPDF_ATTACHMENT, key: &str) -> FPDF_OBJECT_TYPE {
        let c_key = safe_cstring(key);

        unsafe { crate::bindgen::FPDFAttachment_GetValueType(attachment, c_key.as_ptr()) }
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

        unsafe { crate::bindgen::FPDFAttachment_SetStringValue(attachment, c_key.as_ptr(), value) }
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

        unsafe { crate::bindgen::FPDFAttachment_GetStringValue(attachment, c_key.as_ptr(), buffer, buflen) }
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
        unsafe { crate::bindgen::FPDFAttachment_SetFile(attachment, document, contents, len) }
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
        unsafe { crate::bindgen::FPDFAttachment_GetFile(attachment, buffer, buflen, out_buflen) }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FPDFAttachment_GetSubtype(
        &self,
        attachment: FPDF_ATTACHMENT,
        buffer: *mut FPDF_WCHAR,
        buflen: c_ulong,
    ) -> c_ulong {
        unsafe { crate::bindgen::FPDFAttachment_GetSubtype(attachment, buffer, buflen) }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FPDFCatalog_IsTagged(&self, document: FPDF_DOCUMENT) -> FPDF_BOOL {
        unsafe { crate::bindgen::FPDFCatalog_IsTagged(document) }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FPDFCatalog_SetLanguage(&self, document: FPDF_DOCUMENT, language: &str) -> FPDF_BOOL {
        let c_language = safe_cstring(language);

        unsafe { crate::bindgen::FPDFCatalog_SetLanguage(document, c_language.as_ptr()) }
    }
}

impl Drop for StaticPdfiumBindings {
    fn drop(&mut self) {
        self.FPDF_DestroyLibrary();
    }
}
