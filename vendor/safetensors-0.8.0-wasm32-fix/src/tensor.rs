//! Module Containing the most important structures
use crate::lib::{Cow, HashMap, String, ToString, Vec};
use crate::slice::{InvalidSlice, SliceIterator, TensorIndexer};
use core::fmt::Display;
use core::str::Utf8Error;
use serde::{ser::SerializeMap, Deserialize, Deserializer, Serialize, Serializer};
#[cfg(feature = "std")]
use std::{io::Write, path::Path};

const MAX_HEADER_SIZE: usize = 100_000_000;
const N_LEN: usize = size_of::<u64>();

/// Possible errors that could occur while reading
/// A Safetensor file.
#[derive(Debug)]
pub enum SafeTensorError {
    /// The header is an invalid UTF-8 string and cannot be read.
    InvalidHeader(Utf8Error),
    /// The header does contain a valid string, but it is not valid JSON.
    InvalidHeaderDeserialization(serde_json::Error),
    /// The header is large than 100Mo which is considered too large (Might evolve in the future).
    HeaderTooLarge,
    /// The header is smaller than 8 bytes
    HeaderTooSmall,
    /// The header length is invalid
    InvalidHeaderLength,
    /// The tensor name was not found in the archive
    TensorNotFound(String),
    /// Invalid information between shape, dtype and the proposed offsets in the file
    TensorInvalidInfo,
    /// The offsets declared for tensor with name `String` in the header are invalid
    InvalidOffset(String),
    /// IoError
    #[cfg(feature = "std")]
    IoError(std::io::Error),
    /// JSON error
    JsonError(serde_json::Error),
    /// The follow tensor cannot be created because the buffer size doesn't match shape + dtype
    InvalidTensorView(Dtype, Vec<usize>, usize),
    /// The metadata is invalid because the data offsets of the tensor does not
    /// fully cover the buffer part of the file. The last offset **must** be
    /// the end of the file.
    MetadataIncompleteBuffer,
    /// The metadata contains information (shape or shape * dtype size) which lead to an
    /// arithmetic overflow. This is most likely an error in the file.
    ValidationOverflow,
    /// For smaller than 1 byte dtypes, some slices will happen outside of the byte boundary, some special care has to be taken
    /// and standard functions will fail
    MisalignedSlice,
}

#[cfg(feature = "std")]
impl From<std::io::Error> for SafeTensorError {
    fn from(error: std::io::Error) -> SafeTensorError {
        SafeTensorError::IoError(error)
    }
}

impl From<serde_json::Error> for SafeTensorError {
    fn from(error: serde_json::Error) -> SafeTensorError {
        SafeTensorError::JsonError(error)
    }
}

impl Display for SafeTensorError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        use SafeTensorError::*;

        match self {
            InvalidHeader(error) => write!(f, "invalid UTF-8 in header: {error}"),
            InvalidHeaderDeserialization(error) => write!(f, "invalid JSON in header: {error}"),
            JsonError(error) => write!(f, "JSON error: {error}"),
            HeaderTooLarge => write!(f, "header too large"),
            HeaderTooSmall => write!(f, "header too small"),
            InvalidHeaderLength => write!(f, "invalid header length"),
            TensorNotFound(name) => write!(f, "tensor `{name}` not found"),
            TensorInvalidInfo => write!(f, "invalid shape, data type, or offset for tensor"),
            InvalidOffset(name) => write!(f, "invalid offset for tensor `{name}`"),
            #[cfg(feature = "std")]
            IoError(error) => write!(f, "I/O error: {error}"),
            InvalidTensorView(dtype, shape, n_bytes) => {
                write!(f, "tensor of type {dtype} and shape (")?;
                for (i, &dim) in shape.iter().enumerate() {
                    write!(f, "{sep}{dim}", sep = if i == 0 { "" } else { ", " })?;
                }
                write!(f, ") can't be created from {n_bytes} bytes")
            }
            MetadataIncompleteBuffer => write!(f, "incomplete metadata, file not fully covered"),
            ValidationOverflow => write!(f, "overflow computing buffer size from shape and/or element type"),
            MisalignedSlice => write!(f, "The slice is slicing for subbytes dtypes, and the slice does not end up at a byte boundary, this is invalid.")
        }
    }
}

#[cfg(not(feature = "std"))]
impl core::error::Error for SafeTensorError {
    fn source(&self) -> Option<&(dyn core::error::Error + 'static)> {
        match self {
            SafeTensorError::InvalidHeader(source) => Some(source),
            SafeTensorError::JsonError(source) => Some(source),
            SafeTensorError::InvalidHeaderDeserialization(source) => Some(source),
            _ => None,
        }
    }
}

#[cfg(feature = "std")]
impl std::error::Error for SafeTensorError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            SafeTensorError::InvalidHeader(source) => Some(source),
            SafeTensorError::JsonError(source) => Some(source),
            SafeTensorError::InvalidHeaderDeserialization(source) => Some(source),
            SafeTensorError::IoError(source) => Some(source),
            _ => None,
        }
    }
}

struct PreparedData {
    n: u64,
    header_bytes: Vec<u8>,
    offset: usize,
}

/// The trait necessary to enable safetensors to serialize a tensor
/// If you have an owned tensor like this:
///
/// ```rust
/// use safetensors::tensor::{View, Dtype};
/// use std::borrow::Cow;
/// struct Tensor{ dtype: MyDtype, shape: Vec<usize>, data: Vec<u8>}
///
/// # type MyDtype = Dtype;
/// impl<'data> View for &'data Tensor{
///    fn dtype(&self) -> Dtype{
///        self.dtype.into()
///    }
///    fn shape(&self) -> &[usize]{
///         &self.shape
///    }
///    fn data(&self) -> Cow<'_, [u8]>{
///        (&self.data).into()
///    }
///    fn data_len(&self) -> usize{
///        self.data.len()
///    }
/// }
/// ```
///
/// For a borrowed tensor:
///
/// ```rust
/// use safetensors::tensor::{View, Dtype};
/// use std::borrow::Cow;
/// struct Tensor<'data>{ dtype: MyDtype, shape: Vec<usize>, data: &'data[u8]}
///
/// # type MyDtype = Dtype;
/// impl<'data> View for Tensor<'data>{
///    fn dtype(&self) -> Dtype{
///        self.dtype.into()
///    }
///    fn shape(&self) -> &[usize]{
///         &self.shape
///    }
///    fn data(&self) -> Cow<'_, [u8]>{
///        self.data.into()
///    }
///    fn data_len(&self) -> usize{
///        self.data.len()
///    }
/// }
/// ```
///
/// Now if you have some unknown buffer that could be on GPU for instance,
/// you can implement the trait to return an owned local buffer containing the data
/// on CPU (needed to write on disk)
/// ```rust
/// use safetensors::tensor::{View, Dtype};
/// use std::borrow::Cow;
///
/// # type MyDtype = Dtype;
/// # type OpaqueGpu = Vec<u8>;
/// struct Tensor{ dtype: MyDtype, shape: Vec<usize>, data: OpaqueGpu }
///
/// impl View for Tensor{
///    fn dtype(&self) -> Dtype{
///        self.dtype.into()
///    }
///    fn shape(&self) -> &[usize]{
///         &self.shape
///    }
///    fn data(&self) -> Cow<'_, [u8]>{
///        // This copies data from GPU to CPU.
///        let data: Vec<u8> = self.data.to_vec();
///        data.into()
///    }
///    fn data_len(&self) -> usize{
///        let n: usize = self.shape.iter().product();
///        let bytes_per_element = self.dtype.size();
///        n * bytes_per_element
///    }
/// }
/// ```
pub trait View {
    /// The `Dtype` of the tensor
    fn dtype(&self) -> Dtype;
    /// The shape of the tensor
    fn shape(&self) -> &[usize];
    /// The data of the tensor
    fn data(&self) -> Cow<'_, [u8]>;
    /// The length of the data, in bytes.
    /// This is necessary as this might be faster to get than `data().len()`
    /// for instance for tensors residing in GPU.
    fn data_len(&self) -> usize;
}

fn prepare<S, V, I>(
    data: I,
    data_info: Option<HashMap<String, String>>,
) -> Result<(PreparedData, Vec<V>), SafeTensorError>
where
    S: AsRef<str> + Ord + Display,
    V: View,
    I: IntoIterator<Item = (S, V)>,
{
    // Make sure we're sorting by descending dtype alignment
    // Then by name
    let mut data: Vec<_> = data.into_iter().collect();
    data.sort_by(|(lname, left), (rname, right)| {
        right.dtype().cmp(&left.dtype()).then(lname.cmp(rname))
    });

    let mut tensors: Vec<V> = Vec::with_capacity(data.len());
    let mut hmetadata = Vec::with_capacity(data.len());
    let mut offset = 0;

    for (name, tensor) in data {
        let n = tensor.data_len();
        let tensor_info = TensorInfo {
            dtype: tensor.dtype(),
            shape: tensor.shape().to_vec(),
            data_offsets: (offset, offset + n),
        };
        offset += n;
        hmetadata.push((name.to_string(), tensor_info));
        tensors.push(tensor);
    }

    let metadata: Metadata = Metadata::new(data_info, hmetadata)?;
    let mut metadata_buf = serde_json::to_string(&metadata)?.into_bytes();

    // Force alignment to 8 bytes.
    let aligned_metadata_len = metadata_buf.len().next_multiple_of(N_LEN);
    metadata_buf.resize(aligned_metadata_len, b' ');

    Ok((
        PreparedData {
            n: aligned_metadata_len as u64,
            header_bytes: metadata_buf,
            offset,
        },
        tensors,
    ))
}

/// Serialize to an owned byte buffer the dictionnary of tensors.
pub fn serialize<
    S: AsRef<str> + Ord + core::fmt::Display,
    V: View,
    I: IntoIterator<Item = (S, V)>,
>(
    data: I,
    data_info: Option<HashMap<String, String>>,
) -> Result<Vec<u8>, SafeTensorError> {
    let (
        PreparedData {
            n,
            header_bytes,
            offset,
        },
        tensors,
    ) = prepare(data, data_info)?;

    if n > MAX_HEADER_SIZE as u64 {
        return Err(SafeTensorError::HeaderTooLarge);
    }

    let expected_size = N_LEN + header_bytes.len() + offset;
    let mut buffer: Vec<u8> = Vec::with_capacity(expected_size);
    buffer.extend(n.to_le_bytes());
    buffer.extend(header_bytes);

    for tensor in tensors {
        buffer.extend(tensor.data().as_ref());
    }

    Ok(buffer)
}

#[cfg(feature = "std")]
fn buffered_write_to_file<V: View>(
    path: impl AsRef<Path>,
    n: u64,
    header_bytes: &[u8],
    tensors: &[V],
    total_size: usize,
) -> Result<(), SafeTensorError> {
    let path = path.as_ref();
    // Write to a sibling tempfile then rename, so an existing `path` is never
    // truncated under any mmap of it (e.g. tensors returned by `load_file`).
    let parent = path.parent().unwrap_or_else(|| Path::new("."));
    let temp = tempfile::NamedTempFile::new_in(parent)?;

    temp.as_file().set_len(total_size as u64)?;

    // Serialize tensors to a file using direct I/O (bypassing page cache) using F_NOCACHE.
    // This yields ~30% performance improvement.
    #[cfg(target_os = "macos")]
    unsafe {
        use std::os::fd::AsRawFd;

        libc::fcntl(temp.as_file().as_raw_fd(), libc::F_NOCACHE, 1);
    }

    {
        let mut f = std::io::BufWriter::with_capacity(1024 * 1024, temp.as_file());

        f.write_all(n.to_le_bytes().as_ref())?;
        f.write_all(header_bytes)?;

        for tensor in tensors {
            f.write_all(tensor.data().as_ref())?;
        }

        f.flush()?;
    }

    temp.persist(path).map_err(|e| e.error)?;

    Ok(())
}

/// Serialize to a regular file the dictionnary of tensors.
/// Writing directly to file reduces the need to allocate the whole amount to
/// memory.
#[cfg(feature = "std")]
pub fn serialize_to_file<S, V, I>(
    data: I,
    data_info: Option<HashMap<String, String>>,
    filename: &std::path::Path,
) -> Result<(), SafeTensorError>
where
    S: AsRef<str> + Ord + Display,
    V: View,
    I: IntoIterator<Item = (S, V)>,
{
    let (
        PreparedData {
            n,
            header_bytes,
            offset,
            ..
        },
        tensors,
    ) = prepare(data, data_info)?;

    if n > MAX_HEADER_SIZE as u64 {
        return Err(SafeTensorError::HeaderTooLarge);
    }

    let total_size = N_LEN + header_bytes.len() + offset;

    buffered_write_to_file(filename, n, &header_bytes, &tensors, total_size)?;

    Ok(())
}

/// A structure owning some metadata to lookup tensors on a shared `data`
/// byte-buffer (not owned).
#[derive(Debug)]
pub struct SafeTensors<'data> {
    metadata: Metadata,
    data: &'data [u8],
}

impl<'data> SafeTensors<'data> {
    /// Given a byte-buffer representing the whole safetensor file
    /// parses the header, and returns the size of the header + the parsed data.
    pub fn read_metadata(buffer: &'data [u8]) -> Result<(usize, Metadata), SafeTensorError> {
        let buffer_len = buffer.len();
        let Some(header_size_bytes) = buffer.get(..N_LEN) else {
            return Err(SafeTensorError::HeaderTooSmall);
        };
        let arr: [u8; N_LEN] = header_size_bytes
            .try_into()
            .expect("this can't fail due to how `header_size_bytes` is defined above");
        let n: usize = u64::from_le_bytes(arr)
            .try_into()
            .map_err(|_| SafeTensorError::HeaderTooLarge)?;

        if n > MAX_HEADER_SIZE {
            return Err(SafeTensorError::HeaderTooLarge);
        }

        let stop = n
            .checked_add(N_LEN)
            .ok_or(SafeTensorError::InvalidHeaderLength)?;

        // the `.get(start..stop)` returns None if either index is out of bounds,
        // so this implicitly also ensures that `stop <= buffer.len()`.
        let Some(header_bytes) = buffer.get(N_LEN..stop) else {
            return Err(SafeTensorError::InvalidHeaderLength);
        };
        let string = core::str::from_utf8(header_bytes).map_err(SafeTensorError::InvalidHeader)?;
        let metadata: HashMetadata =
            serde_json::from_str(string).map_err(SafeTensorError::InvalidHeaderDeserialization)?;
        let metadata: Metadata = metadata.try_into()?;
        let buffer_end = metadata.validate()?;
        if buffer_end + N_LEN + n != buffer_len {
            return Err(SafeTensorError::MetadataIncompleteBuffer);
        }

        Ok((n, metadata))
    }

    /// Given a byte-buffer representing the whole safetensor file
    /// parses it and returns the Deserialized form (No Tensor allocation).
    ///
    /// ```
    /// use safetensors::SafeTensors;
    /// use memmap2::MmapOptions;
    /// use std::fs::File;
    ///
    /// let filename = "model.safetensors";
    /// # use std::io::Write;
    /// # let serialized = b"<\x00\x00\x00\x00\x00\x00\x00{\"test\":{\"dtype\":\"I32\",\"shape\":[2,2],\"data_offsets\":[0,16]}}\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00";
    /// # File::create(filename).unwrap().write(serialized).unwrap();
    /// let file = File::open(filename).unwrap();
    /// let buffer = unsafe { MmapOptions::new().map(&file).unwrap() };
    /// let tensors = SafeTensors::deserialize(&buffer).unwrap();
    /// let tensor = tensors
    ///         .tensor("test")
    ///         .unwrap();
    /// ```
    pub fn deserialize(buffer: &'data [u8]) -> Result<Self, SafeTensorError> {
        let (n, metadata) = SafeTensors::read_metadata(buffer)?;
        let data = &buffer[N_LEN + n..];
        Ok(Self { metadata, data })
    }

    /// Returns the tensors contained within the SafeTensors.
    /// The tensors returned are merely views and the data is not owned by this
    /// structure.
    pub fn tensors(&self) -> Vec<(String, TensorView<'data>)> {
        let mut tensors = Vec::with_capacity(self.metadata.index_map.len());
        for (name, &index) in &self.metadata.index_map {
            let info = &self.metadata.tensors[index];
            let tensorview = TensorView {
                dtype: info.dtype,
                shape: info.shape.clone(),
                data: &self.data[info.data_offsets.0..info.data_offsets.1],
            };
            tensors.push((name.to_string(), tensorview));
        }
        tensors
    }

    /// Returns an iterator over the tensors contained within the SafeTensors.
    /// The tensors returned are merely views and the data is not owned by this
    /// structure.
    pub fn iter(&self) -> impl Iterator<Item = (&str, TensorView<'data>)> {
        self.metadata.index_map.iter().map(|(name, &idx)| {
            let info = &self.metadata.tensors[idx];
            (
                name.as_str(),
                TensorView {
                    dtype: info.dtype,
                    shape: info.shape.clone(),
                    data: &self.data[info.data_offsets.0..info.data_offsets.1],
                },
            )
        })
    }

    /// Allow the user to get a specific tensor within the SafeTensors.
    /// The tensor returned is merely a view and the data is not owned by this
    /// structure.
    pub fn tensor(&self, tensor_name: &str) -> Result<TensorView<'data>, SafeTensorError> {
        let &index = self
            .metadata
            .index_map
            .get(tensor_name)
            .ok_or_else(|| SafeTensorError::TensorNotFound(tensor_name.to_string()))?;

        let info = self
            .metadata
            .tensors
            .get(index)
            .ok_or_else(|| SafeTensorError::TensorNotFound(tensor_name.to_string()))?;

        Ok(TensorView {
            dtype: info.dtype,
            shape: info.shape.clone(),
            data: &self.data[info.data_offsets.0..info.data_offsets.1],
        })
    }

    /// Return the names of the tensors within the SafeTensors.
    /// These are used as keys to access to the actual tensors, that can be
    /// retrieved using the tensor method.
    pub fn names(&self) -> Vec<&'_ str> {
        self.metadata.index_map.keys().map(String::as_str).collect()
    }

    /// Return how many tensors are currently stored within the SafeTensors.
    #[inline]
    pub fn len(&self) -> usize {
        self.metadata.tensors.len()
    }

    /// Indicate if the SafeTensors contains or not any tensor.
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.metadata.tensors.is_empty()
    }
}

/// The stuct representing the header of safetensor files which allow
/// indexing into the raw byte-buffer array and how to interpret it.
#[derive(Debug, Clone)]
pub struct Metadata {
    metadata: Option<HashMap<String, String>>,
    tensors: Vec<TensorInfo>,
    index_map: HashMap<String, usize>,
}

/// Helper struct used only for serialization and deserialization
#[derive(Serialize, Deserialize)]
struct HashMetadata {
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = "__metadata__")]
    metadata: Option<HashMap<String, String>>,
    #[serde(flatten)]
    tensors: HashMap<String, TensorInfo>,
}

impl TryFrom<HashMetadata> for Metadata {
    type Error = SafeTensorError;
    fn try_from(hashdata: HashMetadata) -> Result<Self, Self::Error> {
        let (metadata, tensors) = (hashdata.metadata, hashdata.tensors);
        let mut tensors: Vec<_> = tensors.into_iter().collect();
        // We need to sort by offsets
        // Previous versions might have a different ordering
        // Than we expect (Not aligned ordered, but purely name ordered,
        // or actually any order).
        tensors.sort_by_key(|(_, left)| left.data_offsets);
        Metadata::new(metadata, tensors)
    }
}

impl<'de> Deserialize<'de> for Metadata {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let hashdata: HashMetadata = HashMetadata::deserialize(deserializer)?;

        let metadata: Metadata = hashdata.try_into().map_err(serde::de::Error::custom)?;
        Ok(metadata)
    }
}

impl Serialize for Metadata {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut names = vec![""; self.index_map.len()];
        for (name, &index) in &self.index_map {
            names[index] = name;
        }

        let length = self.metadata.as_ref().map_or(0, HashMap::len);
        let mut map = serializer.serialize_map(Some(self.tensors.len() + length))?;

        if let Some(metadata) = &self.metadata {
            map.serialize_entry("__metadata__", metadata)?;
        }

        for (name, info) in names.iter().zip(&self.tensors) {
            map.serialize_entry(name, info)?;
        }

        map.end()
    }
}

impl Metadata {
    /// Creates a new metadata structure.
    /// May fail if there is incorrect data in the Tensor Info.
    /// Notably the tensors need to be ordered by increasing data_offsets.
    pub fn new(
        metadata: Option<HashMap<String, String>>,
        tensors: Vec<(String, TensorInfo)>,
    ) -> Result<Self, SafeTensorError> {
        let mut index_map = HashMap::with_capacity(tensors.len());

        let tensors: Vec<_> = tensors
            .into_iter()
            .enumerate()
            .map(|(index, (k, tensor))| {
                index_map.insert(k, index);
                tensor
            })
            .collect();

        let metadata = Self {
            metadata,
            tensors,
            index_map,
        };
        metadata.validate()?;
        Ok(metadata)
    }

    fn validate(&self) -> Result<usize, SafeTensorError> {
        let mut start = 0;
        for (i, info) in self.tensors.iter().enumerate() {
            let (s, e) = info.data_offsets;
            if s != start || e < s {
                let tensor_name = self
                    .index_map
                    .iter()
                    .find_map(|(name, &index)| if index == i { Some(&name[..]) } else { None })
                    .unwrap_or("no_tensor");
                return Err(SafeTensorError::InvalidOffset(tensor_name.to_string()));
            }

            start = e;

            // wasm32 fix: compute the intermediate element/bit count in u128,
            // not `usize`. `nelements * bitsize` can exceed `u32::MAX` (and
            // therefore wasm32's 32-bit `usize`) for tensors well over ~134M
            // elements -- e.g. a DeBERTa-v2-scale token embedding table
            // (vocab_size * hidden_size) can exceed that on its own -- even
            // though the tensor's final BYTE size is a perfectly reasonable
            // few hundred MB. The upstream `usize`-only computation spuriously
            // overflows on wasm32 for such tensors despite `size` below
            // fitting `usize` without any trouble once computed correctly.
            let nelements: u128 = info
                .shape
                .iter()
                .copied()
                .try_fold(1u128, |acc, dim| acc.checked_mul(dim as u128))
                .ok_or(SafeTensorError::ValidationOverflow)?;
            let nbits: u128 = nelements
                .checked_mul(info.dtype.bitsize() as u128)
                .ok_or(SafeTensorError::ValidationOverflow)?;

            if nbits % 8 != 0 {
                return Err(SafeTensorError::MisalignedSlice);
            }
            let size: usize =
                usize::try_from(nbits / 8).map_err(|_| SafeTensorError::ValidationOverflow)?;

            if e - s != size {
                return Err(SafeTensorError::TensorInvalidInfo);
            }
        }
        Ok(start)
    }

    /// Gives back the tensor metadata
    pub fn info(&self, name: &str) -> Option<&TensorInfo> {
        let &index = self.index_map.get(name)?;
        self.tensors.get(index)
    }

    /// Gives back the tensor metadata
    pub fn tensors(&self) -> HashMap<String, &TensorInfo> {
        self.index_map
            .iter()
            .map(|(tensor_name, &index)| (tensor_name.clone(), &self.tensors[index]))
            .collect()
    }

    /// Gives back the tensor names ordered by offset
    pub fn offset_keys(&self) -> Vec<String> {
        let mut index_vec: Vec<_> = self.index_map.iter().collect();
        index_vec.sort_by_key(|a| a.1);
        index_vec.into_iter().map(|a| a.0.clone()).collect()
    }

    /// Gives the size of the content buffer in bytes.
    pub fn data_len(&self) -> usize {
        if let Some(tensor) = self.tensors.last() {
            tensor.data_offsets.1
        } else {
            0
        }
    }

    /// Gives back the tensor metadata
    pub fn metadata(&self) -> &Option<HashMap<String, String>> {
        &self.metadata
    }
}

/// A view of a Tensor within the file.
/// Contains references to data within the full byte-buffer
/// And is thus a readable view of a single tensor
#[derive(Debug, PartialEq, Eq, Clone)]
pub struct TensorView<'data> {
    dtype: Dtype,
    shape: Vec<usize>,
    data: &'data [u8],
}

impl View for &TensorView<'_> {
    fn dtype(&self) -> Dtype {
        self.dtype
    }

    fn shape(&self) -> &[usize] {
        &self.shape
    }

    fn data(&self) -> Cow<'_, [u8]> {
        self.data.into()
    }

    fn data_len(&self) -> usize {
        self.data.len()
    }
}

impl View for TensorView<'_> {
    fn dtype(&self) -> Dtype {
        self.dtype
    }

    fn shape(&self) -> &[usize] {
        &self.shape
    }

    fn data(&self) -> Cow<'_, [u8]> {
        self.data.into()
    }

    fn data_len(&self) -> usize {
        self.data.len()
    }
}

impl<'data> TensorView<'data> {
    /// Create new tensor view
    pub fn new(
        dtype: Dtype,
        shape: Vec<usize>,
        data: &'data [u8],
    ) -> Result<Self, SafeTensorError> {
        let n_elements: usize = shape.iter().product();

        let nbits = n_elements * dtype.bitsize();
        if nbits % 8 != 0 {
            return Err(SafeTensorError::MisalignedSlice);
        }
        let size = nbits
            .checked_div(8)
            .ok_or(SafeTensorError::ValidationOverflow)?;

        if data.len() != size {
            Err(SafeTensorError::InvalidTensorView(dtype, shape, data.len()))
        } else {
            Ok(Self { dtype, shape, data })
        }
    }
    /// The current tensor dtype
    pub fn dtype(&self) -> Dtype {
        self.dtype
    }

    /// The current tensor shape
    pub fn shape(&self) -> &[usize] {
        &self.shape
    }

    /// The current tensor byte-buffer
    pub fn data(&self) -> &'data [u8] {
        self.data
    }

    /// The various pieces of the data buffer according to the asked slice
    pub fn sliced_data(
        &'data self,
        slices: &[TensorIndexer],
    ) -> Result<SliceIterator<'data>, InvalidSlice> {
        SliceIterator::new(self, slices)
    }
}

/// A single tensor information.
/// Endianness is assumed to be little endian
/// Ordering is assumed to be 'C'.
#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct TensorInfo {
    /// The type of each element of the tensor
    pub dtype: Dtype,
    /// The shape of the tensor
    pub shape: Vec<usize>,
    /// The offsets to find the data within the byte-buffer array.
    pub data_offsets: (usize, usize),
}

/// The various available dtypes. They MUST be in increasing alignment order
#[derive(Debug, Deserialize, Serialize, Clone, Copy, PartialEq, Eq, Ord, PartialOrd)]
#[non_exhaustive]
pub enum Dtype {
    /// Boolan type
    BOOL,
    /// MXF4 <https://www.opencompute.org/documents/ocp-microscaling-formats-mx-v1-0-spec-final-pdf>_
    F4,
    /// MXF6 <https://www.opencompute.org/documents/ocp-microscaling-formats-mx-v1-0-spec-final-pdf>_
    #[allow(non_camel_case_types)]
    F6_E2M3,
    /// MXF6 <https://www.opencompute.org/documents/ocp-microscaling-formats-mx-v1-0-spec-final-pdf>_
    #[allow(non_camel_case_types)]
    F6_E3M2,
    /// Unsigned byte
    U8,
    /// Signed byte
    I8,
    /// FP8 <https://arxiv.org/pdf/2209.05433.pdf>_
    #[allow(non_camel_case_types)]
    F8_E5M2,
    /// FP8 <https://arxiv.org/pdf/2209.05433.pdf>_
    #[allow(non_camel_case_types)]
    F8_E4M3,
    /// F8_E8M0 <https://www.opencompute.org/documents/ocp-microscaling-formats-mx-v1-0-spec-final-pdf>_
    #[allow(non_camel_case_types)]
    F8_E8M0,
    /// FP8 E4M3 (FNUZ) <https://arxiv.org/pdf/2206.02915.pdf>_
    #[allow(non_camel_case_types)]
    F8_E4M3FNUZ,
    /// FP8 E5M2 (FNUZ) <https://arxiv.org/pdf/2206.02915.pdf>_
    #[allow(non_camel_case_types)]
    F8_E5M2FNUZ,
    /// Signed integer (16-bit)
    I16,
    /// Unsigned integer (16-bit)
    U16,
    /// Half-precision floating point
    F16,
    /// Brain floating point
    BF16,
    /// Signed integer (32-bit)
    I32,
    /// Unsigned integer (32-bit)
    U32,
    /// Floating point (32-bit)
    F32,
    /// Complex (32-bit parts)
    C64,
    /// Floating point (64-bit)
    F64,
    /// Signed integer (64-bit)
    I64,
    /// Unsigned integer (64-bit)
    U64,
}

impl Dtype {
    /// Gives out the size (in bits) of 1 element of this dtype.
    pub fn bitsize(&self) -> usize {
        match self {
            Dtype::F4 => 4,
            Dtype::F6_E3M2 => 6,
            Dtype::F6_E2M3 => 6,
            Dtype::BOOL => 8,
            Dtype::U8 => 8,
            Dtype::I8 => 8,
            Dtype::F8_E5M2 => 8,
            Dtype::F8_E4M3 => 8,
            Dtype::F8_E8M0 => 8,
            Dtype::F8_E4M3FNUZ => 8,
            Dtype::F8_E5M2FNUZ => 8,
            Dtype::I16 => 16,
            Dtype::U16 => 16,
            Dtype::I32 => 32,
            Dtype::U32 => 32,
            Dtype::I64 => 64,
            Dtype::U64 => 64,
            Dtype::F16 => 16,
            Dtype::BF16 => 16,
            Dtype::F32 => 32,
            Dtype::F64 => 64,
            Dtype::C64 => 64,
        }
    }
    /// Gives out the size (in bytes) of 1 element of this dtype.
    #[deprecated(
        since = "0.6.0",
        note = "Use `bitsize` instead as some elements have smaller than a full byte of width"
    )]
    pub fn size(&self) -> usize {
        self.bitsize() / 8
    }
}

impl Display for Dtype {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.write_str(match *self {
            Dtype::F4 => "F4",
            Dtype::F6_E2M3 => "F6_E2M3",
            Dtype::F6_E3M2 => "F6_E3M2",
            Dtype::BOOL => "BOOL",
            Dtype::I8 => "I8",
            Dtype::U8 => "U8",
            Dtype::F8_E5M2 => "F8_E5M2",
            Dtype::F8_E4M3 => "F8_E4M3",
            Dtype::F8_E8M0 => "F8_E8M0",
            Dtype::F8_E4M3FNUZ => "F8_E4M3FNUZ",
            Dtype::F8_E5M2FNUZ => "F8_E5M2FNUZ",
            Dtype::I16 => "I16",
            Dtype::U16 => "U16",
            Dtype::I32 => "I32",
            Dtype::U32 => "U32",
            Dtype::I64 => "I64",
            Dtype::U64 => "U64",
            Dtype::F16 => "F16",
            Dtype::BF16 => "BF16",
            Dtype::F32 => "F32",
            Dtype::F64 => "F64",
            Dtype::C64 => "C64",
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::slice::IndexOp;
    use proptest::prelude::*;
    #[cfg(not(feature = "std"))]
    extern crate std;
    use std::io::Write;

    const MAX_DIMENSION: usize = 8;
    const MAX_SIZE: usize = 8;
    const MAX_TENSORS: usize = 8;

    fn arbitrary_dtype() -> impl Strategy<Value = Dtype> {
        prop_oneof![
            Just(Dtype::BOOL),
            Just(Dtype::F4),
            Just(Dtype::F6_E3M2),
            Just(Dtype::F6_E2M3),
            Just(Dtype::F8_E5M2),
            Just(Dtype::F8_E4M3),
            Just(Dtype::U8),
            Just(Dtype::I8),
            Just(Dtype::I16),
            Just(Dtype::U16),
            Just(Dtype::I32),
            Just(Dtype::U32),
            Just(Dtype::I64),
            Just(Dtype::U64),
            Just(Dtype::F16),
            Just(Dtype::BF16),
            Just(Dtype::F32),
            Just(Dtype::F64),
            Just(Dtype::C64),
        ]
    }

    fn arbitrary_shape() -> impl Strategy<Value = Vec<usize>> {
        // We do not allow empty shapes or 0 sizes.
        (1..MAX_DIMENSION).prop_flat_map(|length| prop::collection::vec(1..MAX_SIZE, length))
    }

    fn arbitrary_metadata() -> impl Strategy<Value = Metadata> {
        // We generate at least one tensor.
        (1..MAX_TENSORS)
            .prop_flat_map(|size| {
                // Returns a strategy generating `size` data types and shapes.
                (
                    prop::collection::vec(arbitrary_dtype(), size),
                    prop::collection::vec(arbitrary_shape(), size),
                )
            })
            .prop_filter_map("Misaligned slices", |(dtypes, shapes)| {
                // Returns a valid metadata object for a random (length, dtypes, shapes) triple.
                let mut start = 0;
                let tensors: Vec<TensorInfo> = dtypes
                    .iter()
                    .zip(shapes)
                    .flat_map(|(dtype, shape)| {
                        // This cannot overflow because the size of
                        // the vector and elements are so small.
                        let bitlength: usize = shape.iter().product::<usize>() * dtype.bitsize();
                        if bitlength % 8 != 0 {
                            return None;
                        }
                        let length = bitlength.div_ceil(8);
                        let end = start + length;
                        let tensor = TensorInfo {
                            dtype: *dtype,
                            shape,
                            data_offsets: (start, end),
                        };
                        start = end;
                        Some(tensor)
                    })
                    .collect();
                let index_map = (0..tensors.len())
                    .map(|index| (format!("t.{index}"), index))
                    .collect();
                if tensors.is_empty() {
                    None
                } else {
                    Some(Metadata {
                        metadata: None,
                        tensors,
                        index_map,
                    })
                }
            })
    }

    /// This method returns the size of the data corresponding to the metadata. It
    /// assumes that `metadata` contains at least one tensor, and that tensors are
    /// ordered by offset in `metadata.tensors`.
    ///
    /// # Panics
    ///
    /// This method will panic if `metadata` does not contain any tensors.
    fn data_size(metadata: &Metadata) -> usize {
        metadata.tensors.last().unwrap().data_offsets.1
    }

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(20))]

        #[test]
        fn test_indexing(metadata in arbitrary_metadata()) {
            let data = vec![0u8; data_size(&metadata)];
            let tensors = SafeTensors { metadata, data: &data };
            for name in tensors.names() {
                assert!(tensors.tensor(name).is_ok());
            }
        }
        #[test]
        fn test_roundtrip(metadata in arbitrary_metadata()) {
            let data: Vec<u8> = (0..data_size(&metadata)).map(|x| x as u8).collect();
            let before = SafeTensors { metadata, data: &data };
            let tensors = before.tensors();
            let bytes = serialize(tensors.iter().map(|(name, view)| (name.to_string(), view)), None).unwrap();

            let after = SafeTensors::deserialize(&bytes).unwrap();

            // Check that the tensors are the same after deserialization.
            assert_eq!(before.names().len(), after.names().len());
            for name in before.names() {
                let tensor_before = before.tensor(name).unwrap();
                let tensor_after = after.tensor(name).unwrap();
                assert_eq!(tensor_after.data().as_ptr() as usize % tensor_after.dtype().bitsize().div_ceil(8), 0);
                assert_eq!(tensor_before, tensor_after);
            }
        }
    }

    #[test]
    fn test_serialization() {
        let data: Vec<u8> = vec![0.0f32, 1.0, 2.0, 3.0, 4.0, 5.0]
            .into_iter()
            .flat_map(|f| f.to_le_bytes())
            .collect();
        let shape = vec![1, 2, 3];
        let attn_0 = TensorView::new(Dtype::F32, shape, &data).unwrap();
        let metadata: HashMap<String, TensorView> =
            [("attn.0".to_string(), attn_0)].into_iter().collect();

        let out = serialize(&metadata, None).unwrap();
        assert_eq!(
            out,
            [
                64, 0, 0, 0, 0, 0, 0, 0, 123, 34, 97, 116, 116, 110, 46, 48, 34, 58, 123, 34, 100,
                116, 121, 112, 101, 34, 58, 34, 70, 51, 50, 34, 44, 34, 115, 104, 97, 112, 101, 34,
                58, 91, 49, 44, 50, 44, 51, 93, 44, 34, 100, 97, 116, 97, 95, 111, 102, 102, 115,
                101, 116, 115, 34, 58, 91, 48, 44, 50, 52, 93, 125, 125, 0, 0, 0, 0, 0, 0, 128, 63,
                0, 0, 0, 64, 0, 0, 64, 64, 0, 0, 128, 64, 0, 0, 160, 64
            ]
        );
        let _parsed = SafeTensors::deserialize(&out).unwrap();
    }

    #[test]
    fn test_serialization_fp4() {
        let data: Vec<u8> = vec![0u8];
        let shape = vec![1, 2];
        let attn_0 = TensorView::new(Dtype::F4, shape, &data).unwrap();
        let metadata: HashMap<String, TensorView> =
            [("attn.0".to_string(), attn_0)].into_iter().collect();

        let out = serialize(&metadata, None).unwrap();
        assert_eq!(
            out,
            [
                64, 0, 0, 0, 0, 0, 0, 0, 123, 34, 97, 116, 116, 110, 46, 48, 34, 58, 123, 34, 100,
                116, 121, 112, 101, 34, 58, 34, 70, 52, 34, 44, 34, 115, 104, 97, 112, 101, 34, 58,
                91, 49, 44, 50, 93, 44, 34, 100, 97, 116, 97, 95, 111, 102, 102, 115, 101, 116,
                115, 34, 58, 91, 48, 44, 49, 93, 125, 125, 32, 32, 32, 32, 0
            ]
        );
        let parsed = SafeTensors::deserialize(&out).unwrap();
        let tensors: HashMap<_, _> = parsed.tensors().into_iter().collect();
        assert_eq!(tensors, metadata);
    }

    #[test]
    fn test_serialization_fp4_misaligned() {
        let data: Vec<u8> = vec![0u8, 1u8];
        let shape = vec![1, 3];
        let attn_0 = TensorView::new(Dtype::F4, shape, &data);
        assert!(matches!(attn_0, Err(SafeTensorError::MisalignedSlice)));
    }

    #[test]
    fn test_serialization_fp4_invalid() {
        let data: Vec<u8> = vec![0u8, 1u8];
        let shape = vec![1, 2];
        let attn_0 = TensorView::new(Dtype::F4, shape, &data);
        assert!(matches!(
            attn_0,
            Err(SafeTensorError::InvalidTensorView(Dtype::F4, _shape, _size))
        ));
    }

    #[test]
    fn test_empty() {
        let tensors: HashMap<String, TensorView> = HashMap::new();

        let out = serialize(&tensors, None).unwrap();
        assert_eq!(
            out,
            [8, 0, 0, 0, 0, 0, 0, 0, 123, 125, 32, 32, 32, 32, 32, 32]
        );
        let _parsed = SafeTensors::deserialize(&out).unwrap();

        let metadata: Option<HashMap<String, String>> = Some(
            [("framework".to_string(), "pt".to_string())]
                .into_iter()
                .collect(),
        );
        let out = serialize(&tensors, metadata).unwrap();
        assert_eq!(
            out,
            [
                40, 0, 0, 0, 0, 0, 0, 0, 123, 34, 95, 95, 109, 101, 116, 97, 100, 97, 116, 97, 95,
                95, 34, 58, 123, 34, 102, 114, 97, 109, 101, 119, 111, 114, 107, 34, 58, 34, 112,
                116, 34, 125, 125, 32, 32, 32, 32, 32
            ]
        );
        let _parsed = SafeTensors::deserialize(&out).unwrap();
    }

    #[test]
    fn test_serialization_forced_alignement() {
        let data: Vec<u8> = vec![0.0f32, 1.0, 2.0, 3.0, 4.0, 5.0]
            .into_iter()
            .flat_map(|f| f.to_le_bytes())
            .collect();
        let shape = vec![1, 1, 2, 3];
        let attn_0 = TensorView::new(Dtype::F32, shape, &data).unwrap();
        let metadata: HashMap<String, TensorView> =
            // Smaller string to force misalignment compared to previous test.
            [("attn0".to_string(), attn_0)].into_iter().collect();

        let out = serialize(&metadata, None).unwrap();
        assert_eq!(
            out,
            [
                72, 0, 0, 0, 0, 0, 0, 0, 123, 34, 97, 116, 116, 110, 48, 34, 58, 123, 34, 100, 116,
                121, 112, 101, 34, 58, 34, 70, 51, 50, 34, 44, 34, 115, 104, 97, 112, 101, 34, 58,
                91, 49, 44, 49, 44, 50, 44, 51, 93, 44, 34, 100, 97, 116, 97, 95, 111, 102, 102,
                // All the 32 are forcing alignement of the tensor data for casting to f32, f64
                // etc..
                115, 101, 116, 115, 34, 58, 91, 48, 44, 50, 52, 93, 125, 125, 32, 32, 32, 32, 32,
                32, 32, 0, 0, 0, 0, 0, 0, 128, 63, 0, 0, 0, 64, 0, 0, 64, 64, 0, 0, 128, 64, 0, 0,
                160, 64
            ],
        );
        let parsed = SafeTensors::deserialize(&out).unwrap();
        let tensor = parsed.tensor("attn0").unwrap();
        assert_eq!(
            tensor.data().as_ptr() as usize % tensor.dtype().bitsize().div_ceil(8),
            0
        );
    }

    #[test]
    fn test_slicing() {
        let data: Vec<u8> = vec![0.0f32, 1.0, 2.0, 3.0, 4.0, 5.0]
            .into_iter()
            .flat_map(|f| f.to_le_bytes())
            .collect();
        let attn_0 = TensorView {
            dtype: Dtype::F32,
            shape: vec![1, 2, 3],
            data: &data,
        };
        let metadata: HashMap<String, TensorView> =
            [("attn.0".to_string(), attn_0)].into_iter().collect();

        let out = serialize(&metadata, None).unwrap();
        let parsed = SafeTensors::deserialize(&out).unwrap();

        let out_buffer: Vec<u8> = parsed
            .tensor("attn.0")
            .unwrap()
            .slice((.., ..1))
            .unwrap()
            .flat_map(|b| b.to_vec())
            .collect();
        assert_eq!(out_buffer, vec![0u8, 0, 0, 0, 0, 0, 128, 63, 0, 0, 0, 64]);
        assert_eq!(
            out_buffer,
            vec![0.0f32, 1.0, 2.0]
                .into_iter()
                .flat_map(|f| f.to_le_bytes())
                .collect::<Vec<_>>()
        );
        let out_buffer: Vec<u8> = parsed
            .tensor("attn.0")
            .unwrap()
            .slice((.., .., ..1))
            .unwrap()
            .flat_map(|b| b.to_vec())
            .collect();
        assert_eq!(out_buffer, vec![0u8, 0, 0, 0, 0, 0, 64, 64]);
        assert_eq!(
            out_buffer,
            vec![0.0f32, 3.0]
                .into_iter()
                .flat_map(|f| f.to_le_bytes())
                .collect::<Vec<_>>()
        );
    }

    #[test]
    fn test_gpt2() {
        gpt2_like(12, "gpt2");
    }

    #[test]
    fn test_gpt2_tiny() {
        gpt2_like(6, "gpt2_tiny");
    }

    fn gpt2_like(n_heads: usize, model_id: &str) {
        let mut tensors_desc = vec![
            ("wte".to_string(), vec![50257, 768]),
            ("wpe".to_string(), vec![1024, 768]),
        ];
        for i in 0..n_heads {
            tensors_desc.push((format!("h.{i}.ln_1.weight"), vec![768]));
            tensors_desc.push((format!("h.{i}.ln_1.bias"), vec![768]));
            tensors_desc.push((format!("h.{i}.attn.bias"), vec![1, 1, 1024, 1024]));
            tensors_desc.push((format!("h.{i}.attn.c_attn.weight"), vec![768, 2304]));
            tensors_desc.push((format!("h.{i}.attn.c_attn.bias"), vec![2304]));
            tensors_desc.push((format!("h.{i}.attn.c_proj.weight"), vec![768, 768]));
            tensors_desc.push((format!("h.{i}.attn.c_proj.bias"), vec![768]));
            tensors_desc.push((format!("h.{i}.ln_2.weight"), vec![768]));
            tensors_desc.push((format!("h.{i}.ln_2.bias"), vec![768]));
            tensors_desc.push((format!("h.{i}.mlp.c_fc.weight"), vec![768, 3072]));
            tensors_desc.push((format!("h.{i}.mlp.c_fc.bias"), vec![3072]));
            tensors_desc.push((format!("h.{i}.mlp.c_proj.weight"), vec![3072, 768]));
            tensors_desc.push((format!("h.{i}.mlp.c_proj.bias"), vec![768]));
        }
        tensors_desc.push(("ln_f.weight".to_string(), vec![768]));
        tensors_desc.push(("ln_f.bias".to_string(), vec![768]));

        let dtype = Dtype::F32;
        let nbits: usize = tensors_desc
            .iter()
            .map(|(_, shape)| shape.iter().product::<usize>())
            .sum::<usize>()
            * dtype.bitsize();
        if nbits % 8 != 0 {
            panic!("Misaligned slice");
        }
        let n = nbits
            .checked_div(8)
            .ok_or(SafeTensorError::ValidationOverflow)
            .unwrap(); // 4
        let all_data = vec![0; n];
        let mut metadata = HashMap::with_capacity(tensors_desc.len());
        let mut offset = 0;
        for (name, shape) in tensors_desc {
            let n: usize = shape.iter().product();
            let buffer = &all_data[offset..offset + (n * dtype.bitsize()) / 8];
            let tensor = TensorView::new(dtype, shape, buffer).unwrap();
            metadata.insert(name, tensor);
            offset += n;
        }

        let filename = format!("./out_{model_id}.safetensors");

        let out = serialize(&metadata, None).unwrap();
        std::fs::write(&filename, out).unwrap();
        let raw = std::fs::read(&filename).unwrap();
        let _deserialized = SafeTensors::deserialize(&raw).unwrap();
        std::fs::remove_file(&filename).unwrap();

        // File api
        #[cfg(feature = "std")]
        {
            serialize_to_file(&metadata, None, std::path::Path::new(&filename)).unwrap();
            let raw = std::fs::read(&filename).unwrap();
            let _deserialized = SafeTensors::deserialize(&raw).unwrap();
            std::fs::remove_file(&filename).unwrap();
        }
    }

    #[cfg(all(feature = "std", unix))]
    #[test]
    fn test_serialize_to_file_same_path_with_active_mmap() {
        // Regression test for #762: on Linux, `serialize_to_file` to a path that
        // is currently mmap'd (e.g. by views the caller is about to serialize)
        // would truncate the destination and zero those mmap pages before the
        // bytes were read for writing, producing a zero-filled output file.
        //
        // FIXME: Windows is unsupported. Rename-over-active-mmap requires POSIX
        // inode semantics; `MoveFileExW` returns ERROR_ACCESS_DENIED when the
        // destination has open handles or active section objects. Callers on
        // Windows must drop mmaps of `path` before calling `serialize_to_file`
        // with the same `path`.
        use memmap2::MmapOptions;

        let filename = std::env::temp_dir().join(format!(
            "safetensors_test_762_{}_{:?}.safetensors",
            std::process::id(),
            std::thread::current().id()
        ));
        let _ = std::fs::remove_file(&filename);

        let bytes: Vec<u8> = [1.0f32, 2.0, 3.0]
            .into_iter()
            .flat_map(|f| f.to_le_bytes())
            .collect();
        let view = TensorView::new(Dtype::F32, vec![3], &bytes).unwrap();
        let metadata: HashMap<String, TensorView> = [("w".to_string(), view)].into_iter().collect();
        serialize_to_file(&metadata, None, &filename).unwrap();

        // Mmap the file, deserialize, and re-save to the same path with views
        // that borrow from the mmap.
        {
            let file = std::fs::File::open(&filename).unwrap();
            let mmap = unsafe { MmapOptions::new().map(&file).unwrap() };
            let safetensors = SafeTensors::deserialize(&mmap).unwrap();
            let mmap_tensors = safetensors.tensors();
            serialize_to_file(
                mmap_tensors.iter().map(|(k, v)| (k.as_str(), v)),
                None,
                &filename,
            )
            .unwrap();
        }

        let raw = std::fs::read(&filename).unwrap();
        let reloaded = SafeTensors::deserialize(&raw).unwrap();
        assert_eq!(reloaded.tensor("w").unwrap().data(), bytes.as_slice());
        std::fs::remove_file(&filename).unwrap();
    }

    #[test]
    fn test_empty_shapes_allowed() {
        let serialized = b"8\x00\x00\x00\x00\x00\x00\x00{\"test\":{\"dtype\":\"I32\",\"shape\":[],\"data_offsets\":[0,4]}}\x00\x00\x00\x00";

        let loaded = SafeTensors::deserialize(serialized).unwrap();
        assert_eq!(loaded.names(), vec!["test"]);
        let tensor = loaded.tensor("test").unwrap();
        assert!(tensor.shape().is_empty());
        assert_eq!(tensor.dtype(), Dtype::I32);
        // 4 bytes
        assert_eq!(tensor.data(), b"\0\0\0\0");
    }

    #[test]
    fn test_deserialization() {
        let serialized = b"<\x00\x00\x00\x00\x00\x00\x00{\"test\":{\"dtype\":\"I32\",\"shape\":[2,2],\"data_offsets\":[0,16]}}\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00";

        let loaded = SafeTensors::deserialize(serialized).unwrap();

        assert_eq!(loaded.len(), 1);
        assert_eq!(loaded.names(), vec!["test"]);
        let tensor = loaded.tensor("test").unwrap();
        assert_eq!(tensor.shape(), vec![2, 2]);
        assert_eq!(tensor.dtype(), Dtype::I32);
        // 16 bytes
        assert_eq!(tensor.data(), b"\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0");
    }

    #[test]
    fn test_lifetimes() {
        let serialized = b"<\x00\x00\x00\x00\x00\x00\x00{\"test\":{\"dtype\":\"I32\",\"shape\":[2,2],\"data_offsets\":[0,16]}}\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00";

        let tensor = {
            let loaded = SafeTensors::deserialize(serialized).unwrap();
            loaded.tensor("test").unwrap()
        };

        assert_eq!(tensor.shape(), vec![2, 2]);
        assert_eq!(tensor.dtype(), Dtype::I32);
        // 16 bytes
        assert_eq!(tensor.data(), b"\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0");
    }

    #[test]
    fn test_json_attack() {
        let mut tensors = HashMap::new();
        let dtype = Dtype::F32;
        let shape = vec![2, 2];
        let data_offsets = (0, 16);
        for i in 0..10 {
            tensors.insert(
                format!("weight_{i}"),
                TensorInfo {
                    dtype,
                    shape: shape.clone(),
                    data_offsets,
                },
            );
        }

        let metadata = HashMetadata {
            metadata: None,
            tensors,
        };
        let serialized = serde_json::to_string(&metadata).unwrap();
        let serialized = serialized.as_bytes();

        let n = serialized.len();

        let filename = "out.safetensors";
        let mut f = std::io::BufWriter::new(std::fs::File::create(filename).unwrap());
        f.write_all(n.to_le_bytes().as_ref()).unwrap();
        f.write_all(serialized).unwrap();
        f.write_all(b"\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0").unwrap();
        f.flush().unwrap();

        let reloaded = std::fs::read(filename).unwrap();
        match SafeTensors::deserialize(&reloaded) {
            Err(SafeTensorError::InvalidOffset(_)) => {
                // Yes we have the correct error, name of the tensor is random though
            }
            Err(err) => panic!("Unexpected error {err:?}"),
            Ok(_) => panic!("This should not be able to be deserialized"),
        }
    }

    #[test]
    fn test_metadata_incomplete_buffer() {
        let serialized = b"<\x00\x00\x00\x00\x00\x00\x00{\"test\":{\"dtype\":\"I32\",\"shape\":[2,2],\"data_offsets\":[0,16]}}\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00extra_bogus_data_for_polyglot_file";

        match SafeTensors::deserialize(serialized) {
            Err(SafeTensorError::MetadataIncompleteBuffer) => {
                // Yes we have the correct error
            }
            _ => panic!("This should not be able to be deserialized"),
        }

        // Missing data in the buffer
        let serialized = b"<\x00\x00\x00\x00\x00\x00\x00{\"test\":{\"dtype\":\"I32\",\"shape\":[2,2],\"data_offsets\":[0,16]}}\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00"; // <--- missing 2 bytes

        match SafeTensors::deserialize(serialized) {
            Err(SafeTensorError::MetadataIncompleteBuffer) => {
                // Yes we have the correct error
            }
            _ => panic!("This should not be able to be deserialized"),
        }
    }

    #[test]
    fn test_header_too_large() {
        let serialized = b"<\x00\x00\x00\x00\xff\xff\xff{\"test\":{\"dtype\":\"I32\",\"shape\":[2,2],\"data_offsets\":[0,16]}}\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00";

        match SafeTensors::deserialize(serialized) {
            Err(SafeTensorError::HeaderTooLarge) => {
                // Yes we have the correct error
            }
            _ => panic!("This should not be able to be deserialized"),
        }
    }

    #[test]
    fn test_header_too_small() {
        let serialized = b"";
        match SafeTensors::deserialize(serialized) {
            Err(SafeTensorError::HeaderTooSmall) => {
                // Yes we have the correct error
            }
            _ => panic!("This should not be able to be deserialized"),
        }
    }

    #[test]
    fn test_invalid_header_length() {
        let serialized = b"<\x00\x00\x00\x00\x00\x00\x00";
        match SafeTensors::deserialize(serialized) {
            Err(SafeTensorError::InvalidHeaderLength) => {
                // Yes we have the correct error
            }
            _ => panic!("This should not be able to be deserialized"),
        }
    }

    #[test]
    fn test_invalid_header_non_utf8() {
        let serialized = b"\x01\x00\x00\x00\x00\x00\x00\x00\xff";
        match SafeTensors::deserialize(serialized) {
            Err(SafeTensorError::InvalidHeader(_)) => {
                // Yes we have the correct error
            }
            _ => panic!("This should not be able to be deserialized"),
        }
    }

    #[test]
    fn test_invalid_header_not_json() {
        let serialized = b"\x01\x00\x00\x00\x00\x00\x00\x00{";
        match SafeTensors::deserialize(serialized) {
            Err(SafeTensorError::InvalidHeaderDeserialization(_)) => {
                // Yes we have the correct error
            }
            _ => panic!("This should not be able to be deserialized"),
        }
    }

    #[test]
    /// Test that the JSON header may be trailing-padded with JSON whitespace characters.
    fn test_whitespace_padded_header() {
        let serialized = b"\x06\x00\x00\x00\x00\x00\x00\x00{}\x0D\x20\x09\x0A";
        let loaded = SafeTensors::deserialize(serialized).unwrap();
        assert_eq!(loaded.len(), 0);
    }

    #[test]
    /// Test that the JSON header may be leading-padded with JSON whitespace characters.
    /// This is intentional: writers may pad the header to align the data section to a
    /// page boundary, so readers must tolerate leading whitespace.
    fn test_whitespace_leading_padded_header() {
        let serialized = b"\x06\x00\x00\x00\x00\x00\x00\x00\x09\x0A{}\x0D\x20";
        let loaded = SafeTensors::deserialize(serialized).unwrap();
        assert_eq!(loaded.len(), 0);
    }

    #[test]
    fn test_zero_sized_tensor() {
        let serialized = b"<\x00\x00\x00\x00\x00\x00\x00{\"test\":{\"dtype\":\"I32\",\"shape\":[2,0],\"data_offsets\":[0, 0]}}";
        let loaded = SafeTensors::deserialize(serialized).unwrap();

        assert_eq!(loaded.names(), vec!["test"]);
        let tensor = loaded.tensor("test").unwrap();
        assert_eq!(tensor.shape(), vec![2, 0]);
        assert_eq!(tensor.dtype(), Dtype::I32);
        assert_eq!(tensor.data(), b"");
    }

    #[test]
    fn test_invalid_info() {
        let serialized = b"<\x00\x00\x00\x00\x00\x00\x00{\"test\":{\"dtype\":\"I32\",\"shape\":[2,2],\"data_offsets\":[0, 4]}}";
        match SafeTensors::deserialize(serialized) {
            Err(SafeTensorError::TensorInvalidInfo) => {
                // Yes we have the correct error
            }
            something => panic!("This should not be able to be deserialized got {something:?}"),
        }
    }

    #[test]
    fn test_validation_overflow() {
        // u64::MAX =  18_446_744_073_709_551_615u64
        // Overflow the shape calculation.
        let serialized = b"O\x00\x00\x00\x00\x00\x00\x00{\"test\":{\"dtype\":\"I32\",\"shape\":[2,18446744073709551614],\"data_offsets\":[0,16]}}\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00";
        match SafeTensors::deserialize(serialized) {
            Err(SafeTensorError::ValidationOverflow) => {
                // Yes we have the correct error
            }
            _ => panic!("This should not be able to be deserialized"),
        }
        // u64::MAX =  18_446_744_073_709_551_615u64
        // Overflow the num_elements * total shape.
        let serialized = b"N\x00\x00\x00\x00\x00\x00\x00{\"test\":{\"dtype\":\"I32\",\"shape\":[2,9223372036854775807],\"data_offsets\":[0,16]}}\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00";
        match SafeTensors::deserialize(serialized) {
            Err(SafeTensorError::ValidationOverflow) => {
                // Yes we have the correct error
            }
            _ => panic!("This should not be able to be deserialized"),
        }
    }

    #[test]
    fn test_invalid_header_size_serialization() {
        let mut data_info = HashMap::<String, String>::new();
        let tensors: HashMap<String, TensorView> = HashMap::new();

        // a char is 1 byte in utf-8, so we can just repeat 'a' to get large metadata
        let very_large_metadata = "a".repeat(MAX_HEADER_SIZE);
        data_info.insert("very_large_metadata".to_string(), very_large_metadata);
        match serialize(&tensors, Some(data_info)) {
            Err(SafeTensorError::HeaderTooLarge) => {
                // Yes we have the correct error
            }
            _ => panic!("This should not be able to be serialized"),
        }
    }
}
