//! Kitty graphics image loading.

use std::ffi::{CString, OsString};
use std::fs::{self, File};
use std::io::{Read, Seek, SeekFrom};
use std::os::unix::ffi::{OsStrExt, OsStringExt};
use std::path::{Path, PathBuf};
use std::ptr;
use std::slice;
use std::time::Instant;

use flate2::read::ZlibDecoder;

use super::graphics_command::{
    Command, Display, Quiet, TransmissionCompression, TransmissionFormat, TransmissionMedium,
};

pub(crate) const MAX_DIMENSION: u32 = 10_000;
pub(crate) const MAX_IMAGE_SIZE: usize = 400 * 1024 * 1024;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ImageLoadError {
    InvalidData,
    DecompressionFailed,
    DimensionsRequired,
    DimensionsTooLarge,
    TemporaryFileNotInTempDir,
    TemporaryFileNotNamedCorrectly,
    UnsupportedFormat,
    UnsupportedMedium,
    OutOfMemory,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct LoadingImageLimits {
    pub(crate) file: bool,
    pub(crate) temporary_file: bool,
    pub(crate) shared_memory: bool,
}

#[derive(Debug, PartialEq, Eq)]
pub(crate) struct LoadingImage {
    pub(crate) image: Image,
    pub(crate) data: Vec<u8>,
    pub(crate) display: Option<Display>,
    pub(crate) quiet: Quiet,
}

#[derive(Debug, PartialEq, Eq)]
pub(crate) struct Image {
    pub(crate) id: u32,
    pub(crate) number: u32,
    pub(crate) width: u32,
    pub(crate) height: u32,
    pub(crate) format: TransmissionFormat,
    pub(crate) compression: TransmissionCompression,
    pub(crate) data: Vec<u8>,
    pub(crate) transmit_time: Option<Instant>,
    pub(crate) implicit_id: bool,
}

impl LoadingImageLimits {
    pub(crate) const ALL: Self = Self {
        file: true,
        temporary_file: true,
        shared_memory: true,
    };

    pub(crate) const DIRECT: Self = Self {
        file: false,
        temporary_file: false,
        shared_memory: false,
    };
}

impl Default for Image {
    fn default() -> Self {
        Self {
            id: 0,
            number: 0,
            width: 0,
            height: 0,
            format: TransmissionFormat::Rgb,
            compression: TransmissionCompression::None,
            data: Vec::new(),
            transmit_time: None,
            implicit_id: false,
        }
    }
}

impl Image {
    pub(crate) fn without_data(&self) -> Self {
        Self {
            id: self.id,
            number: self.number,
            width: self.width,
            height: self.height,
            format: self.format,
            compression: self.compression,
            data: Vec::new(),
            transmit_time: self.transmit_time,
            implicit_id: self.implicit_id,
        }
    }
}

impl LoadingImage {
    pub(crate) fn init(
        command: &Command,
        limits: LoadingImageLimits,
    ) -> Result<Self, ImageLoadError> {
        let transmission = command.transmission().ok_or(ImageLoadError::InvalidData)?;

        let mut result = Self {
            image: Image {
                id: transmission.image_id,
                number: transmission.image_number,
                width: transmission.width,
                height: transmission.height,
                compression: transmission.compression,
                format: transmission.format,
                implicit_id: transmission.image_id == 0 && transmission.image_number == 0,
                ..Image::default()
            },
            data: Vec::new(),
            display: command.display(),
            quiet: command.quiet,
        };

        match transmission.medium {
            TransmissionMedium::Direct => {
                result.add_data(&command.data)?;
                Ok(result)
            }
            TransmissionMedium::File => {
                if !limits.file || png_without_decoder(transmission.format) {
                    return Err(ImageLoadError::UnsupportedMedium);
                }
                result.read_file(transmission, &command.data, false)?;
                Ok(result)
            }
            TransmissionMedium::TemporaryFile => {
                if !limits.temporary_file || png_without_decoder(transmission.format) {
                    return Err(ImageLoadError::UnsupportedMedium);
                }
                result.read_file(transmission, &command.data, true)?;
                Ok(result)
            }
            TransmissionMedium::SharedMemory => {
                if !limits.shared_memory || png_without_decoder(transmission.format) {
                    return Err(ImageLoadError::UnsupportedMedium);
                }
                result.read_shared_memory(transmission, &command.data)?;
                Ok(result)
            }
        }
    }

    pub(crate) fn add_data(&mut self, data: &[u8]) -> Result<(), ImageLoadError> {
        if data.is_empty() {
            return Ok(());
        }

        let new_len = self
            .data
            .len()
            .checked_add(data.len())
            .ok_or(ImageLoadError::InvalidData)?;
        if new_len > MAX_IMAGE_SIZE {
            return Err(ImageLoadError::InvalidData);
        }

        self.data
            .try_reserve(data.len())
            .map_err(|_| ImageLoadError::OutOfMemory)?;
        self.data.extend_from_slice(data);
        Ok(())
    }

    pub(crate) fn complete(mut self) -> Result<Image, ImageLoadError> {
        self.decompress()?;

        if self.image.format == TransmissionFormat::Png {
            self.decode_png()?;
        }

        if self.image.width == 0 || self.image.height == 0 {
            return Err(ImageLoadError::DimensionsRequired);
        }
        if self.image.width > MAX_DIMENSION || self.image.height > MAX_DIMENSION {
            return Err(ImageLoadError::DimensionsTooLarge);
        }

        let bytes_per_pixel = self
            .image
            .format
            .bytes_per_pixel()
            .ok_or(ImageLoadError::UnsupportedFormat)?;
        let expected_len = (self.image.width as usize)
            .checked_mul(self.image.height as usize)
            .and_then(|pixels| pixels.checked_mul(bytes_per_pixel))
            .ok_or(ImageLoadError::InvalidData)?;
        if self.data.len() != expected_len {
            return Err(ImageLoadError::InvalidData);
        }

        self.image.transmit_time = Some(Instant::now());
        self.image.data = std::mem::take(&mut self.data);
        Ok(self.image)
    }

    fn decode_png(&mut self) -> Result<(), ImageLoadError> {
        let decoded = crate::sys_decode_png(&self.data)?;
        self.data = decoded.data;
        self.image.width = decoded.width;
        self.image.height = decoded.height;
        self.image.format = TransmissionFormat::Rgba;
        Ok(())
    }

    fn decompress(&mut self) -> Result<(), ImageLoadError> {
        match self.image.compression {
            TransmissionCompression::None => Ok(()),
            TransmissionCompression::ZlibDeflate => self.decompress_zlib(),
        }
    }

    fn decompress_zlib(&mut self) -> Result<(), ImageLoadError> {
        let mut decoder = ZlibDecoder::new(self.data.as_slice());
        let mut output = Vec::new();
        let mut buf = [0u8; 8192];

        loop {
            let read = decoder
                .read(&mut buf)
                .map_err(|_| ImageLoadError::DecompressionFailed)?;
            if read == 0 {
                break;
            }

            let new_len = output
                .len()
                .checked_add(read)
                .ok_or(ImageLoadError::InvalidData)?;
            if new_len > MAX_IMAGE_SIZE {
                return Err(ImageLoadError::InvalidData);
            }

            output
                .try_reserve(read)
                .map_err(|_| ImageLoadError::OutOfMemory)?;
            output.extend_from_slice(&buf[..read]);
        }

        self.data = output;
        self.image.compression = TransmissionCompression::None;
        Ok(())
    }

    fn read_file(
        &mut self,
        transmission: super::graphics_command::Transmission,
        path_data: &[u8],
        temporary: bool,
    ) -> Result<(), ImageLoadError> {
        if path_data.contains(&0) {
            return Err(ImageLoadError::InvalidData);
        }

        let path = PathBuf::from(OsString::from_vec(path_data.to_vec()));
        let path = fs::canonicalize(path).map_err(|_| ImageLoadError::InvalidData)?;
        if is_unsafe_path(&path) {
            return Err(ImageLoadError::InvalidData);
        }

        let _cleanup = if temporary {
            if !is_path_in_temp_dir(&path) {
                return Err(ImageLoadError::TemporaryFileNotInTempDir);
            }
            if !path
                .as_os_str()
                .as_bytes()
                .windows(b"tty-graphics-protocol".len())
                .any(|part| part == b"tty-graphics-protocol")
            {
                return Err(ImageLoadError::TemporaryFileNotNamedCorrectly);
            }
            Some(TemporaryFileCleanup { path: path.clone() })
        } else {
            None
        };

        let mut file = File::open(&path).map_err(|_| ImageLoadError::InvalidData)?;
        let metadata = file.metadata().map_err(|_| ImageLoadError::InvalidData)?;
        if !metadata.file_type().is_file() {
            return Err(ImageLoadError::InvalidData);
        }

        if transmission.offset > 0 {
            file.seek(SeekFrom::Start(u64::from(transmission.offset)))
                .map_err(|_| ImageLoadError::InvalidData)?;
        }

        let limit = if transmission.size > 0 {
            usize::try_from(transmission.size)
                .unwrap_or(MAX_IMAGE_SIZE)
                .min(MAX_IMAGE_SIZE)
        } else {
            MAX_IMAGE_SIZE
        };
        let mut data = Vec::new();
        file.take(limit as u64)
            .read_to_end(&mut data)
            .map_err(|_| ImageLoadError::InvalidData)?;
        self.data = data;
        Ok(())
    }

    fn read_shared_memory(
        &mut self,
        transmission: super::graphics_command::Transmission,
        name: &[u8],
    ) -> Result<(), ImageLoadError> {
        let name = CString::new(name).map_err(|_| ImageLoadError::InvalidData)?;
        let fd = unsafe { libc::shm_open(name.as_ptr(), libc::O_RDONLY, 0) };
        if fd < 0 {
            return Err(ImageLoadError::InvalidData);
        }
        let handle = SharedMemoryHandle { fd, name };

        let mut stat = std::mem::MaybeUninit::<libc::stat>::uninit();
        let result = unsafe { libc::fstat(handle.fd, stat.as_mut_ptr()) };
        if result != 0 {
            return Err(ImageLoadError::InvalidData);
        }
        let stat = unsafe { stat.assume_init() };
        if stat.st_size <= 0 {
            return Err(ImageLoadError::InvalidData);
        }
        let stat_size = usize::try_from(stat.st_size).map_err(|_| ImageLoadError::InvalidData)?;
        let expected_size = if transmission.format == TransmissionFormat::Png {
            stat_size
        } else {
            expected_raw_size(transmission)?
        };
        if stat_size < expected_size {
            return Err(ImageLoadError::InvalidData);
        }

        let start =
            usize::try_from(transmission.offset).map_err(|_| ImageLoadError::InvalidData)?;
        if start > expected_size {
            return Err(ImageLoadError::InvalidData);
        }
        let end = if transmission.size > 0 {
            let size =
                usize::try_from(transmission.size).map_err(|_| ImageLoadError::InvalidData)?;
            start
                .checked_add(size)
                .ok_or(ImageLoadError::InvalidData)?
                .min(expected_size)
        } else {
            expected_size
        };
        if end < start {
            return Err(ImageLoadError::InvalidData);
        }

        let mapping = SharedMemoryMap::new(handle.fd, stat_size)?;
        let data = unsafe { mapping.bytes(start, end - start) };
        self.data
            .try_reserve(data.len())
            .map_err(|_| ImageLoadError::OutOfMemory)?;
        self.data.extend_from_slice(data);
        Ok(())
    }
}

struct TemporaryFileCleanup {
    path: PathBuf,
}

impl Drop for TemporaryFileCleanup {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.path);
    }
}

struct SharedMemoryHandle {
    fd: libc::c_int,
    name: CString,
}

impl Drop for SharedMemoryHandle {
    fn drop(&mut self) {
        unsafe {
            let _ = libc::shm_unlink(self.name.as_ptr());
            let _ = libc::close(self.fd);
        }
    }
}

struct SharedMemoryMap {
    ptr: *mut libc::c_void,
    len: usize,
}

impl SharedMemoryMap {
    fn new(fd: libc::c_int, len: usize) -> Result<Self, ImageLoadError> {
        let ptr = unsafe {
            libc::mmap(
                ptr::null_mut(),
                len,
                libc::PROT_READ,
                libc::MAP_SHARED,
                fd,
                0,
            )
        };
        if ptr == libc::MAP_FAILED {
            return Err(ImageLoadError::InvalidData);
        }
        Ok(Self { ptr, len })
    }

    unsafe fn bytes(&self, start: usize, len: usize) -> &[u8] {
        debug_assert!(start <= self.len);
        debug_assert!(len <= self.len - start);
        slice::from_raw_parts(self.ptr.cast::<u8>().add(start), len)
    }
}

impl Drop for SharedMemoryMap {
    fn drop(&mut self) {
        unsafe {
            let _ = libc::munmap(self.ptr, self.len);
        }
    }
}

fn expected_raw_size(
    transmission: super::graphics_command::Transmission,
) -> Result<usize, ImageLoadError> {
    let bytes_per_pixel = transmission
        .format
        .bytes_per_pixel()
        .ok_or(ImageLoadError::UnsupportedFormat)?;
    (transmission.width as usize)
        .checked_mul(transmission.height as usize)
        .and_then(|pixels| pixels.checked_mul(bytes_per_pixel))
        .ok_or(ImageLoadError::InvalidData)
}

fn png_without_decoder(format: TransmissionFormat) -> bool {
    format == TransmissionFormat::Png && !crate::sys_has_decode_png()
}

fn is_unsafe_path(path: &Path) -> bool {
    let bytes = path.as_os_str().as_bytes();
    bytes.starts_with(b"/proc/")
        || bytes.starts_with(b"/sys/")
        || (bytes.starts_with(b"/dev/") && !bytes.starts_with(b"/dev/shm/"))
}

fn is_path_in_temp_dir(path: &Path) -> bool {
    let path_bytes = path.as_os_str().as_bytes();
    if path_bytes.starts_with(b"/tmp") || path_bytes.starts_with(b"/dev/shm") {
        return true;
    }

    let temp = std::env::temp_dir();
    if path.starts_with(&temp) {
        return true;
    }

    if let Ok(temp) = fs::canonicalize(temp) {
        if path.starts_with(temp) {
            return true;
        }
    }

    false
}

#[cfg(test)]
mod tests {
    use super::super::graphics_command::{
        CommandControl, Transmission, TransmissionCompression, TransmissionFormat,
        TransmissionMedium,
    };
    use super::*;
    use std::fs;
    use std::os::raw::c_void;
    use std::os::unix::ffi::OsStrExt;
    use std::ptr;
    use std::sync::atomic::{AtomicU64, Ordering};
    use std::sync::MutexGuard;

    const RGB_NONE_20X15: &[u8] = include_bytes!(
        "../../../../vendor/ghostty/src/terminal/kitty/testdata/image-rgb-none-20x15-2147483647-raw.data"
    );
    const RGB_ZLIB_128X96: &[u8] = include_bytes!(
        "../../../../vendor/ghostty/src/terminal/kitty/testdata/image-rgb-zlib_deflate-128x96-2147483647-raw.data"
    );
    static TEST_NAME_COUNTER: AtomicU64 = AtomicU64::new(0);

    fn next_test_name_id() -> u64 {
        TEST_NAME_COUNTER.fetch_add(1, Ordering::Relaxed)
    }

    fn transmit_command(transmission: Transmission, data: &[u8]) -> Command {
        Command {
            control: CommandControl::Transmit(transmission),
            quiet: Quiet::No,
            data: data.to_vec(),
        }
    }

    struct SysDecodeGuard {
        _guard: MutexGuard<'static, ()>,
    }

    impl SysDecodeGuard {
        fn without_png_decoder() -> Self {
            let guard = crate::SYS_TEST_LOCK.lock().unwrap();
            assert_eq!(
                crate::roastty_sys_set(crate::ROASTTY_SYS_OPT_DECODE_PNG, ptr::null()),
                crate::ROASTTY_SUCCESS
            );
            Self { _guard: guard }
        }

        fn with_png_decoder(
            callback: unsafe extern "C" fn(
                *mut c_void,
                *const crate::RoasttyAllocator,
                *const u8,
                usize,
                *mut crate::RoasttySysImage,
            ) -> bool,
        ) -> Self {
            let guard = crate::SYS_TEST_LOCK.lock().unwrap();
            assert_eq!(
                crate::roastty_sys_set(
                    crate::ROASTTY_SYS_OPT_DECODE_PNG,
                    callback as *const c_void
                ),
                crate::ROASTTY_SUCCESS
            );
            Self { _guard: guard }
        }
    }

    impl Drop for SysDecodeGuard {
        fn drop(&mut self) {
            let _ = crate::roastty_sys_set(crate::ROASTTY_SYS_OPT_DECODE_PNG, ptr::null());
        }
    }

    unsafe extern "C" fn decode_png_rgba_1x1(
        _userdata: *mut c_void,
        allocator: *const crate::RoasttyAllocator,
        _data: *const u8,
        _data_len: usize,
        out: *mut crate::RoasttySysImage,
    ) -> bool {
        write_decoded_png(allocator, out, 1, 1, &[9, 8, 7, 6])
    }

    unsafe extern "C" fn decode_png_fails(
        _userdata: *mut c_void,
        _allocator: *const crate::RoasttyAllocator,
        _data: *const u8,
        _data_len: usize,
        _out: *mut crate::RoasttySysImage,
    ) -> bool {
        false
    }

    unsafe extern "C" fn decode_png_null_zero(
        _userdata: *mut c_void,
        _allocator: *const crate::RoasttyAllocator,
        _data: *const u8,
        _data_len: usize,
        out: *mut crate::RoasttySysImage,
    ) -> bool {
        out.write(crate::RoasttySysImage {
            width: 0,
            height: 0,
            data: ptr::null_mut(),
            data_len: 0,
        });
        true
    }

    unsafe extern "C" fn decode_png_null_nonzero(
        _userdata: *mut c_void,
        _allocator: *const crate::RoasttyAllocator,
        _data: *const u8,
        _data_len: usize,
        out: *mut crate::RoasttySysImage,
    ) -> bool {
        out.write(crate::RoasttySysImage {
            width: 1,
            height: 1,
            data: ptr::null_mut(),
            data_len: 4,
        });
        true
    }

    unsafe extern "C" fn decode_png_oversized_for_test(
        _userdata: *mut c_void,
        allocator: *const crate::RoasttyAllocator,
        _data: *const u8,
        _data_len: usize,
        out: *mut crate::RoasttySysImage,
    ) -> bool {
        write_decoded_png(allocator, out, 1, 2, &[1, 2, 3, 4, 5, 6, 7, 8])
    }

    unsafe extern "C" fn decode_png_zero_width(
        _userdata: *mut c_void,
        allocator: *const crate::RoasttyAllocator,
        _data: *const u8,
        _data_len: usize,
        out: *mut crate::RoasttySysImage,
    ) -> bool {
        write_decoded_png(allocator, out, 0, 1, &[1, 2, 3, 4])
    }

    unsafe extern "C" fn decode_png_length_mismatch(
        _userdata: *mut c_void,
        allocator: *const crate::RoasttyAllocator,
        _data: *const u8,
        _data_len: usize,
        out: *mut crate::RoasttySysImage,
    ) -> bool {
        write_decoded_png(allocator, out, 2, 1, &[1, 2, 3, 4])
    }

    unsafe fn write_decoded_png(
        allocator: *const crate::RoasttyAllocator,
        out: *mut crate::RoasttySysImage,
        width: u32,
        height: u32,
        data: &[u8],
    ) -> bool {
        let ptr = crate::roastty_alloc(allocator, data.len());
        if ptr.is_null() {
            return false;
        }
        ptr::copy_nonoverlapping(data.as_ptr(), ptr, data.len());
        out.write(crate::RoasttySysImage {
            width,
            height,
            data: ptr,
            data_len: data.len(),
        });
        true
    }

    struct TestDir {
        path: PathBuf,
    }

    impl TestDir {
        fn temp() -> Self {
            Self::in_base(std::env::temp_dir())
        }

        fn target() -> Self {
            Self::in_base(PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../target"))
        }

        fn in_base(base: PathBuf) -> Self {
            let id = next_test_name_id();
            let path = base.join(format!(
                "roastty-kitty-graphics-image-{}-{id}",
                std::process::id()
            ));
            fs::create_dir_all(&path).unwrap();
            Self { path }
        }

        fn write(&self, name: &str, data: &[u8]) -> PathBuf {
            let path = self.path.join(name);
            fs::write(&path, data).unwrap();
            fs::canonicalize(path).unwrap()
        }

        fn mkdir(&self, name: &str) -> PathBuf {
            let path = self.path.join(name);
            fs::create_dir(&path).unwrap();
            fs::canonicalize(path).unwrap()
        }
    }

    impl Drop for TestDir {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.path);
        }
    }

    fn path_bytes(path: &Path) -> Vec<u8> {
        path.as_os_str().as_bytes().to_vec()
    }

    struct SharedMemoryObject {
        name: CString,
    }

    impl SharedMemoryObject {
        fn new(data: &[u8]) -> Self {
            let id = next_test_name_id();
            let name = CString::new(format!("/rt{:x}{id:x}", std::process::id())).unwrap();
            let fd = unsafe {
                libc::shm_open(
                    name.as_ptr(),
                    libc::O_CREAT | libc::O_EXCL | libc::O_RDWR,
                    0o600,
                )
            };
            assert!(fd >= 0);
            assert_eq!(unsafe { libc::ftruncate(fd, data.len() as libc::off_t) }, 0);
            if !data.is_empty() {
                let ptr = unsafe {
                    libc::mmap(
                        ptr::null_mut(),
                        data.len(),
                        libc::PROT_READ | libc::PROT_WRITE,
                        libc::MAP_SHARED,
                        fd,
                        0,
                    )
                };
                assert_ne!(ptr, libc::MAP_FAILED);
                unsafe {
                    ptr::copy_nonoverlapping(data.as_ptr(), ptr.cast::<u8>(), data.len());
                    assert_eq!(libc::munmap(ptr, data.len()), 0);
                }
            }
            assert_eq!(unsafe { libc::close(fd) }, 0);
            Self { name }
        }

        fn name_bytes(&self) -> &[u8] {
            self.name.as_bytes()
        }

        fn exists(&self) -> bool {
            let fd = unsafe { libc::shm_open(self.name.as_ptr(), libc::O_RDONLY, 0) };
            if fd < 0 {
                return false;
            }
            unsafe {
                let _ = libc::close(fd);
            }
            true
        }
    }

    impl Drop for SharedMemoryObject {
        fn drop(&mut self) {
            unsafe {
                let _ = libc::shm_unlink(self.name.as_ptr());
            }
        }
    }

    #[test]
    fn kitty_graphics_image_load_with_invalid_rgb_data_allowed_at_init() {
        let command = transmit_command(
            Transmission {
                format: TransmissionFormat::Rgb,
                width: 1,
                height: 1,
                image_id: 31,
                ..Transmission::default()
            },
            b"AAAA",
        );

        let loading = LoadingImage::init(&command, LoadingImageLimits::DIRECT).unwrap();
        assert_eq!(loading.image.id, 31);
        assert_eq!(loading.data, b"AAAA");
    }

    #[test]
    fn kitty_graphics_image_too_wide_errors_on_complete() {
        let command = transmit_command(
            Transmission {
                format: TransmissionFormat::Rgb,
                width: MAX_DIMENSION + 1,
                height: 1,
                image_id: 31,
                ..Transmission::default()
            },
            b"AAAA",
        );

        let loading = LoadingImage::init(&command, LoadingImageLimits::DIRECT).unwrap();
        assert_eq!(loading.complete(), Err(ImageLoadError::DimensionsTooLarge));
    }

    #[test]
    fn kitty_graphics_image_too_tall_errors_on_complete() {
        let command = transmit_command(
            Transmission {
                format: TransmissionFormat::Rgb,
                width: 1,
                height: MAX_DIMENSION + 1,
                image_id: 31,
                ..Transmission::default()
            },
            b"AAAA",
        );

        let loading = LoadingImage::init(&command, LoadingImageLimits::DIRECT).unwrap();
        assert_eq!(loading.complete(), Err(ImageLoadError::DimensionsTooLarge));
    }

    #[test]
    fn kitty_graphics_image_rgb_zlib_compressed_direct() {
        let command = transmit_command(
            Transmission {
                format: TransmissionFormat::Rgb,
                medium: TransmissionMedium::Direct,
                compression: TransmissionCompression::ZlibDeflate,
                width: 128,
                height: 96,
                image_id: 31,
                ..Transmission::default()
            },
            RGB_ZLIB_128X96,
        );

        let loading = LoadingImage::init(&command, LoadingImageLimits::DIRECT).unwrap();
        let image = loading.complete().unwrap();
        assert_eq!(image.compression, TransmissionCompression::None);
        assert_eq!(image.data.len(), 128 * 96 * 3);
    }

    #[test]
    fn kitty_graphics_image_rgb_uncompressed_direct() {
        let command = transmit_command(
            Transmission {
                format: TransmissionFormat::Rgb,
                medium: TransmissionMedium::Direct,
                compression: TransmissionCompression::None,
                width: 20,
                height: 15,
                image_id: 31,
                ..Transmission::default()
            },
            RGB_NONE_20X15,
        );

        let loading = LoadingImage::init(&command, LoadingImageLimits::DIRECT).unwrap();
        let image = loading.complete().unwrap();
        assert_eq!(image.compression, TransmissionCompression::None);
        assert_eq!(image.data.len(), 20 * 15 * 3);
    }

    #[test]
    fn kitty_graphics_image_rgb_zlib_compressed_direct_chunked() {
        let command = transmit_command(
            Transmission {
                format: TransmissionFormat::Rgb,
                medium: TransmissionMedium::Direct,
                compression: TransmissionCompression::ZlibDeflate,
                width: 128,
                height: 96,
                image_id: 31,
                more_chunks: true,
                ..Transmission::default()
            },
            &RGB_ZLIB_128X96[..1024],
        );

        let mut loading = LoadingImage::init(&command, LoadingImageLimits::DIRECT).unwrap();
        for chunk in RGB_ZLIB_128X96[1024..].chunks(1024) {
            loading.add_data(chunk).unwrap();
        }

        let image = loading.complete().unwrap();
        assert_eq!(image.compression, TransmissionCompression::None);
        assert_eq!(image.data.len(), 128 * 96 * 3);
    }

    #[test]
    fn kitty_graphics_image_rgb_zlib_compressed_direct_chunked_zero_initial_chunk() {
        let command = transmit_command(
            Transmission {
                format: TransmissionFormat::Rgb,
                medium: TransmissionMedium::Direct,
                compression: TransmissionCompression::ZlibDeflate,
                width: 128,
                height: 96,
                image_id: 31,
                more_chunks: true,
                ..Transmission::default()
            },
            b"",
        );

        let mut loading = LoadingImage::init(&command, LoadingImageLimits::DIRECT).unwrap();
        for chunk in RGB_ZLIB_128X96.chunks(1024) {
            loading.add_data(chunk).unwrap();
        }

        let image = loading.complete().unwrap();
        assert_eq!(image.compression, TransmissionCompression::None);
        assert_eq!(image.data.len(), 128 * 96 * 3);
    }

    #[test]
    fn kitty_graphics_image_direct_medium_always_allowed_by_limits() {
        let command = transmit_command(
            Transmission {
                format: TransmissionFormat::Rgb,
                medium: TransmissionMedium::Direct,
                width: 1,
                height: 1,
                image_id: 31,
                ..Transmission::default()
            },
            b"AAAA",
        );

        let loading = LoadingImage::init(&command, LoadingImageLimits::DIRECT).unwrap();
        assert_eq!(loading.image.id, 31);
    }

    #[test]
    fn kitty_graphics_image_final_byte_length_mismatch_errors() {
        let command = transmit_command(
            Transmission {
                format: TransmissionFormat::Rgb,
                medium: TransmissionMedium::Direct,
                width: 2,
                height: 2,
                image_id: 31,
                ..Transmission::default()
            },
            b"AAAA",
        );

        let loading = LoadingImage::init(&command, LoadingImageLimits::DIRECT).unwrap();
        assert_eq!(loading.complete(), Err(ImageLoadError::InvalidData));
    }

    #[test]
    fn kitty_graphics_image_missing_dimensions_error() {
        let command = transmit_command(
            Transmission {
                format: TransmissionFormat::Rgb,
                medium: TransmissionMedium::Direct,
                image_id: 31,
                ..Transmission::default()
            },
            b"AAAA",
        );

        let loading = LoadingImage::init(&command, LoadingImageLimits::DIRECT).unwrap();
        assert_eq!(loading.complete(), Err(ImageLoadError::DimensionsRequired));
    }

    #[test]
    fn kitty_graphics_image_malformed_zlib_errors() {
        let command = transmit_command(
            Transmission {
                format: TransmissionFormat::Rgb,
                medium: TransmissionMedium::Direct,
                compression: TransmissionCompression::ZlibDeflate,
                width: 1,
                height: 1,
                image_id: 31,
                ..Transmission::default()
            },
            b"not zlib",
        );

        let loading = LoadingImage::init(&command, LoadingImageLimits::DIRECT).unwrap();
        assert_eq!(loading.complete(), Err(ImageLoadError::DecompressionFailed));
    }

    #[test]
    fn kitty_graphics_image_png_direct_deferred() {
        let _guard = SysDecodeGuard::without_png_decoder();
        let command = transmit_command(
            Transmission {
                format: TransmissionFormat::Png,
                medium: TransmissionMedium::Direct,
                width: 1,
                height: 1,
                image_id: 31,
                ..Transmission::default()
            },
            b"not a png",
        );

        let loading = LoadingImage::init(&command, LoadingImageLimits::DIRECT).unwrap();
        assert_eq!(loading.complete(), Err(ImageLoadError::UnsupportedFormat));
    }

    #[test]
    fn kitty_graphics_image_png_direct_decodes_through_sys_callback() {
        let _guard = SysDecodeGuard::with_png_decoder(decode_png_rgba_1x1);
        let command = transmit_command(
            Transmission {
                format: TransmissionFormat::Png,
                medium: TransmissionMedium::Direct,
                width: 99,
                height: 88,
                image_id: 31,
                ..Transmission::default()
            },
            b"fake png",
        );

        let image = LoadingImage::init(&command, LoadingImageLimits::DIRECT)
            .unwrap()
            .complete()
            .unwrap();
        assert_eq!(image.width, 1);
        assert_eq!(image.height, 1);
        assert_eq!(image.format, TransmissionFormat::Rgba);
        assert_eq!(image.data, [9, 8, 7, 6]);
    }

    #[test]
    fn kitty_graphics_image_png_callback_errors_and_malformed_output() {
        for (callback, expected) in [
            (
                decode_png_fails
                    as unsafe extern "C" fn(
                        *mut c_void,
                        *const crate::RoasttyAllocator,
                        *const u8,
                        usize,
                        *mut crate::RoasttySysImage,
                    ) -> bool,
                ImageLoadError::InvalidData,
            ),
            (decode_png_null_zero, ImageLoadError::InvalidData),
            (decode_png_null_nonzero, ImageLoadError::InvalidData),
            (decode_png_zero_width, ImageLoadError::DimensionsRequired),
            (decode_png_length_mismatch, ImageLoadError::InvalidData),
        ] {
            let _guard = SysDecodeGuard::with_png_decoder(callback);
            let command = transmit_command(
                Transmission {
                    format: TransmissionFormat::Png,
                    medium: TransmissionMedium::Direct,
                    image_id: 31,
                    ..Transmission::default()
                },
                b"fake png",
            );

            let loading = LoadingImage::init(&command, LoadingImageLimits::DIRECT).unwrap();
            assert_eq!(loading.complete(), Err(expected));
        }
    }

    #[test]
    fn kitty_graphics_image_png_callback_oversized_output_fails_before_copy() {
        let _guard = SysDecodeGuard::with_png_decoder(decode_png_oversized_for_test);
        assert!(matches!(
            crate::sys_decode_png_with_limit_for_test(b"fake png", 4),
            Err(ImageLoadError::InvalidData)
        ));
    }

    #[test]
    fn kitty_graphics_image_file_medium_blocked_and_allowed_by_limits() {
        let dir = TestDir::temp();
        let path = dir.write("image.data", RGB_NONE_20X15);
        let command = transmit_command(
            Transmission {
                format: TransmissionFormat::Rgb,
                medium: TransmissionMedium::File,
                width: 20,
                height: 15,
                image_id: 31,
                ..Transmission::default()
            },
            &path_bytes(&path),
        );

        assert_eq!(
            LoadingImage::init(&command, LoadingImageLimits::DIRECT),
            Err(ImageLoadError::UnsupportedMedium)
        );

        let loading = LoadingImage::init(
            &command,
            LoadingImageLimits {
                file: true,
                temporary_file: false,
                shared_memory: false,
            },
        )
        .unwrap();
        let image = loading.complete().unwrap();
        assert_eq!(image.data, RGB_NONE_20X15);
        assert!(path.exists());
    }

    #[test]
    fn kitty_graphics_image_temporary_file_medium_blocked_and_allowed_by_limits() {
        let dir = TestDir::temp();
        let path = dir.write("tty-graphics-protocol-image.data", RGB_NONE_20X15);
        let command = transmit_command(
            Transmission {
                format: TransmissionFormat::Rgb,
                medium: TransmissionMedium::TemporaryFile,
                width: 20,
                height: 15,
                image_id: 31,
                ..Transmission::default()
            },
            &path_bytes(&path),
        );

        assert_eq!(
            LoadingImage::init(
                &command,
                LoadingImageLimits {
                    file: true,
                    temporary_file: false,
                    shared_memory: true,
                },
            ),
            Err(ImageLoadError::UnsupportedMedium)
        );
        assert!(path.exists());

        let loading = LoadingImage::init(
            &command,
            LoadingImageLimits {
                file: false,
                temporary_file: true,
                shared_memory: false,
            },
        )
        .unwrap();
        let image = loading.complete().unwrap();
        assert_eq!(image.data, RGB_NONE_20X15);
        assert!(!path.exists());
    }

    #[test]
    fn kitty_graphics_image_temporary_file_validation_controls_cleanup() {
        let dir = TestDir::temp();
        let wrong_name = dir.write("image.data", RGB_NONE_20X15);
        let command = transmit_command(
            Transmission {
                format: TransmissionFormat::Rgb,
                medium: TransmissionMedium::TemporaryFile,
                width: 20,
                height: 15,
                image_id: 31,
                ..Transmission::default()
            },
            &path_bytes(&wrong_name),
        );
        assert_eq!(
            LoadingImage::init(&command, LoadingImageLimits::ALL),
            Err(ImageLoadError::TemporaryFileNotNamedCorrectly)
        );
        assert!(wrong_name.exists());

        let outside = TestDir::target();
        let outside_path = outside.write("tty-graphics-protocol-image.data", RGB_NONE_20X15);
        let command = transmit_command(
            Transmission {
                format: TransmissionFormat::Rgb,
                medium: TransmissionMedium::TemporaryFile,
                width: 20,
                height: 15,
                image_id: 32,
                ..Transmission::default()
            },
            &path_bytes(&outside_path),
        );
        assert_eq!(
            LoadingImage::init(&command, LoadingImageLimits::ALL),
            Err(ImageLoadError::TemporaryFileNotInTempDir)
        );
        assert!(outside_path.exists());

        let invalid = dir.write("tty-graphics-protocol-invalid.data", b"AAAA");
        let command = transmit_command(
            Transmission {
                format: TransmissionFormat::Rgb,
                medium: TransmissionMedium::TemporaryFile,
                width: 20,
                height: 15,
                image_id: 33,
                ..Transmission::default()
            },
            &path_bytes(&invalid),
        );
        let loading = LoadingImage::init(&command, LoadingImageLimits::ALL).unwrap();
        assert_eq!(loading.complete(), Err(ImageLoadError::InvalidData));
        assert!(!invalid.exists());
    }

    #[test]
    fn kitty_graphics_image_file_media_offset_size_and_invalid_paths() {
        let dir = TestDir::temp();
        let mut padded = b"XX".to_vec();
        padded.extend_from_slice(RGB_NONE_20X15);
        padded.extend_from_slice(b"YY");
        let path = dir.write("image.data", &padded);
        let command = transmit_command(
            Transmission {
                format: TransmissionFormat::Rgb,
                medium: TransmissionMedium::File,
                width: 20,
                height: 15,
                image_id: 31,
                offset: 2,
                size: u32::try_from(RGB_NONE_20X15.len()).unwrap(),
                ..Transmission::default()
            },
            &path_bytes(&path),
        );
        let image = LoadingImage::init(&command, LoadingImageLimits::ALL)
            .unwrap()
            .complete()
            .unwrap();
        assert_eq!(image.data, RGB_NONE_20X15);

        let directory = dir.mkdir("directory.data");
        let command = transmit_command(
            Transmission {
                format: TransmissionFormat::Rgb,
                medium: TransmissionMedium::File,
                width: 1,
                height: 1,
                image_id: 32,
                ..Transmission::default()
            },
            &path_bytes(&directory),
        );
        assert_eq!(
            LoadingImage::init(&command, LoadingImageLimits::ALL),
            Err(ImageLoadError::InvalidData)
        );

        let command = transmit_command(
            Transmission {
                format: TransmissionFormat::Rgb,
                medium: TransmissionMedium::File,
                width: 1,
                height: 1,
                image_id: 33,
                ..Transmission::default()
            },
            b"/tmp/roastty\0image",
        );
        assert_eq!(
            LoadingImage::init(&command, LoadingImageLimits::ALL),
            Err(ImageLoadError::InvalidData)
        );

        let command = transmit_command(
            Transmission {
                format: TransmissionFormat::Rgb,
                medium: TransmissionMedium::File,
                width: 1,
                height: 1,
                image_id: 34,
                ..Transmission::default()
            },
            b"/dev/null",
        );
        assert_eq!(
            LoadingImage::init(&command, LoadingImageLimits::ALL),
            Err(ImageLoadError::InvalidData)
        );
    }

    #[test]
    fn kitty_graphics_image_non_direct_png_and_shared_memory_remain_deferred() {
        let _guard = SysDecodeGuard::without_png_decoder();
        let dir = TestDir::temp();
        let png_temp = dir.write("tty-graphics-protocol-image.png", b"not a png");

        for medium in [TransmissionMedium::File, TransmissionMedium::TemporaryFile] {
            let command = transmit_command(
                Transmission {
                    format: TransmissionFormat::Png,
                    medium,
                    width: 0,
                    height: 0,
                    image_id: 31,
                    ..Transmission::default()
                },
                &path_bytes(&png_temp),
            );
            assert_eq!(
                LoadingImage::init(&command, LoadingImageLimits::ALL),
                Err(ImageLoadError::UnsupportedMedium)
            );
        }
        assert!(png_temp.exists());

        let shm = SharedMemoryObject::new(b"not a png");
        let command = transmit_command(
            Transmission {
                format: TransmissionFormat::Png,
                medium: TransmissionMedium::SharedMemory,
                image_id: 31,
                ..Transmission::default()
            },
            shm.name_bytes(),
        );
        assert_eq!(
            LoadingImage::init(&command, LoadingImageLimits::ALL),
            Err(ImageLoadError::UnsupportedMedium)
        );
        assert!(shm.exists());
    }

    #[test]
    fn kitty_graphics_image_non_direct_png_decodes_when_sys_callback_is_installed() {
        let _guard = SysDecodeGuard::with_png_decoder(decode_png_rgba_1x1);
        let dir = TestDir::temp();

        for (medium, name) in [
            (TransmissionMedium::File, "image.png"),
            (
                TransmissionMedium::TemporaryFile,
                "tty-graphics-protocol-image.png",
            ),
        ] {
            let path = dir.write(name, b"fake png");
            let command = transmit_command(
                Transmission {
                    format: TransmissionFormat::Png,
                    medium,
                    image_id: 31,
                    ..Transmission::default()
                },
                &path_bytes(&path),
            );
            let image = LoadingImage::init(&command, LoadingImageLimits::ALL)
                .unwrap()
                .complete()
                .unwrap();
            assert_eq!(image.format, TransmissionFormat::Rgba);
            assert_eq!(image.data, [9, 8, 7, 6]);
            assert_eq!(path.exists(), medium == TransmissionMedium::File);
        }

        let shm = SharedMemoryObject::new(b"fake png");
        let command = transmit_command(
            Transmission {
                format: TransmissionFormat::Png,
                medium: TransmissionMedium::SharedMemory,
                image_id: 32,
                ..Transmission::default()
            },
            shm.name_bytes(),
        );
        let image = LoadingImage::init(&command, LoadingImageLimits::ALL)
            .unwrap()
            .complete()
            .unwrap();
        assert_eq!(image.format, TransmissionFormat::Rgba);
        assert_eq!(image.data, [9, 8, 7, 6]);
        assert!(!shm.exists());
    }

    #[test]
    fn kitty_graphics_image_shared_memory_blocked_and_allowed_by_limits() {
        let shm = SharedMemoryObject::new(RGB_NONE_20X15);
        let command = transmit_command(
            Transmission {
                format: TransmissionFormat::Rgb,
                medium: TransmissionMedium::SharedMemory,
                width: 20,
                height: 15,
                image_id: 31,
                ..Transmission::default()
            },
            shm.name_bytes(),
        );

        assert_eq!(
            LoadingImage::init(&command, LoadingImageLimits::DIRECT),
            Err(ImageLoadError::UnsupportedMedium)
        );
        assert!(shm.exists());

        let image = LoadingImage::init(&command, LoadingImageLimits::ALL)
            .unwrap()
            .complete()
            .unwrap();
        assert_eq!(image.data, RGB_NONE_20X15);
        assert!(!shm.exists());
    }

    #[test]
    fn kitty_graphics_image_shared_memory_unlinks_after_open_failures() {
        let empty = SharedMemoryObject::new(b"");
        let command = transmit_command(
            Transmission {
                format: TransmissionFormat::Rgba,
                medium: TransmissionMedium::SharedMemory,
                width: 1,
                height: 1,
                image_id: 31,
                ..Transmission::default()
            },
            empty.name_bytes(),
        );
        assert_eq!(
            LoadingImage::init(&command, LoadingImageLimits::ALL),
            Err(ImageLoadError::InvalidData)
        );
        assert!(!empty.exists());

        let too_small = SharedMemoryObject::new(b"ABC");
        let command = transmit_command(
            Transmission {
                format: TransmissionFormat::Rgba,
                medium: TransmissionMedium::SharedMemory,
                width: 10_000,
                height: 1,
                image_id: 32,
                ..Transmission::default()
            },
            too_small.name_bytes(),
        );
        assert_eq!(
            LoadingImage::init(&command, LoadingImageLimits::ALL),
            Err(ImageLoadError::InvalidData)
        );
        assert!(!too_small.exists());

        let overflow = SharedMemoryObject::new(b"AAAA");
        let command = transmit_command(
            Transmission {
                format: TransmissionFormat::Rgba,
                medium: TransmissionMedium::SharedMemory,
                width: u32::MAX,
                height: u32::MAX,
                image_id: 33,
                ..Transmission::default()
            },
            overflow.name_bytes(),
        );
        assert_eq!(
            LoadingImage::init(&command, LoadingImageLimits::ALL),
            Err(ImageLoadError::InvalidData)
        );
        assert!(!overflow.exists());
    }

    #[test]
    fn kitty_graphics_image_shared_memory_invalid_names_and_ranges() {
        let command = transmit_command(
            Transmission {
                format: TransmissionFormat::Rgba,
                medium: TransmissionMedium::SharedMemory,
                width: 1,
                height: 1,
                image_id: 31,
                ..Transmission::default()
            },
            b"/missing-roastty-kitty-shared-memory",
        );
        assert_eq!(
            LoadingImage::init(&command, LoadingImageLimits::ALL),
            Err(ImageLoadError::InvalidData)
        );

        let command = transmit_command(
            Transmission {
                format: TransmissionFormat::Rgba,
                medium: TransmissionMedium::SharedMemory,
                width: 1,
                height: 1,
                image_id: 32,
                ..Transmission::default()
            },
            b"/roastty\0kitty",
        );
        assert_eq!(
            LoadingImage::init(&command, LoadingImageLimits::ALL),
            Err(ImageLoadError::InvalidData)
        );

        let ranged = SharedMemoryObject::new(&[1, 2, 3, 4]);
        let command = transmit_command(
            Transmission {
                format: TransmissionFormat::Rgba,
                medium: TransmissionMedium::SharedMemory,
                width: 1,
                height: 1,
                image_id: 33,
                offset: 1,
                size: 2,
                ..Transmission::default()
            },
            ranged.name_bytes(),
        );
        let loading = LoadingImage::init(&command, LoadingImageLimits::ALL).unwrap();
        assert_eq!(loading.data, [2, 3]);
        assert_eq!(loading.complete(), Err(ImageLoadError::InvalidData));
        assert!(!ranged.exists());

        let invalid_range = SharedMemoryObject::new(&[1, 2, 3, 4]);
        let command = transmit_command(
            Transmission {
                format: TransmissionFormat::Rgba,
                medium: TransmissionMedium::SharedMemory,
                width: 1,
                height: 1,
                image_id: 34,
                offset: 5,
                ..Transmission::default()
            },
            invalid_range.name_bytes(),
        );
        assert_eq!(
            LoadingImage::init(&command, LoadingImageLimits::ALL),
            Err(ImageLoadError::InvalidData)
        );
        assert!(!invalid_range.exists());
    }

    #[test]
    fn kitty_graphics_image_without_data_preserves_metadata_only() {
        let image = Image {
            id: 31,
            number: 7,
            width: 20,
            height: 15,
            format: TransmissionFormat::Rgb,
            compression: TransmissionCompression::None,
            data: RGB_NONE_20X15.to_vec(),
            transmit_time: Some(Instant::now()),
            implicit_id: true,
        };

        let without_data = image.without_data();
        assert_eq!(without_data.id, image.id);
        assert_eq!(without_data.number, image.number);
        assert_eq!(without_data.width, image.width);
        assert_eq!(without_data.height, image.height);
        assert_eq!(without_data.format, image.format);
        assert_eq!(without_data.compression, image.compression);
        assert_eq!(without_data.transmit_time, image.transmit_time);
        assert_eq!(without_data.implicit_id, image.implicit_id);
        assert!(without_data.data.is_empty());
        assert_eq!(image.data, RGB_NONE_20X15);
    }
}
