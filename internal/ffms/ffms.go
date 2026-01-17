// Package ffms provides CGO bindings to FFMS2 for video frame extraction.
package ffms

/*
#cgo pkg-config: ffms2
#include <ffms.h>
#include <stdlib.h>
#include <string.h>

#define ERR_BUF_SIZE 1024

// Helper to create an error info struct with C-allocated buffer
static FFMS_ErrorInfo* create_error_info() {
	FFMS_ErrorInfo* err = (FFMS_ErrorInfo*)malloc(sizeof(FFMS_ErrorInfo));
	err->Buffer = (char*)malloc(ERR_BUF_SIZE);
	err->BufferSize = ERR_BUF_SIZE;
	err->Buffer[0] = '\0';
	return err;
}

// Helper to free error info struct
static void free_error_info(FFMS_ErrorInfo* err) {
	if (err) {
		free(err->Buffer);
		free(err);
	}
}

// Helper to get error message from FFMS_ErrorInfo
static const char* get_error_message(FFMS_ErrorInfo* err) {
	return err->Buffer;
}
*/
import "C"

import (
	"fmt"
	"sync"
	"unsafe"
)

var initOnce sync.Once

// Init initializes the FFMS2 library. Safe to call multiple times.
func Init() {
	initOnce.Do(func() {
		C.FFMS_Init(0, 0)
	})
}

// VidIdx wraps an FFMS_Index pointer.
type VidIdx struct {
	ptr       *C.FFMS_Index
	videoPath string
}

// VidSrc wraps an FFMS_VideoSource pointer.
type VidSrc struct {
	ptr *C.FFMS_VideoSource
}

// VidInf contains video properties and HDR metadata.
type VidInf struct {
	Width                   uint32
	Height                  uint32
	FPSNum                  uint32
	FPSDen                  uint32
	Frames                  int
	ColorPrimaries          *int32
	TransferCharacteristics *int32
	MatrixCoefficients      *int32
	Is10Bit                 bool
	MasteringDisplay        *string
	ContentLight            *string
	PixelFormat             int
}

// DecodeStrat represents the decoding strategy for frame extraction.
type DecodeStrat int

const (
	// B10Fast is fast 10-bit decoding without cropping
	B10Fast DecodeStrat = iota
	// B10Stride is 10-bit decoding with stride handling
	B10Stride
	// B8Fast is fast 8-bit decoding without cropping
	B8Fast
	// B8Stride is 8-bit decoding with stride handling
	B8Stride
	// B10CropFast is fast 10-bit decoding with cropping
	B10CropFast
	// B10CropStride is 10-bit decoding with cropping and stride handling
	B10CropStride
	// B8CropFast is fast 8-bit decoding with cropping
	B8CropFast
	// B8CropStride is 8-bit decoding with cropping and stride handling
	B8CropStride
)

// CropCalc contains crop calculation parameters for frame extraction.
type CropCalc struct {
	NewW     uint32 // Cropped width
	NewH     uint32 // Cropped height
	YStride  int    // Source Y plane stride
	UVStride int    // Source UV plane stride
	YStart   int    // Byte offset to first Y pixel
	YLen     int    // Bytes per row of cropped Y
	UVOff    int    // Byte offset to first UV pixel
	UVLen    int    // Bytes per row of cropped UV
	CropV    uint32 // Vertical crop amount (top/bottom)
	CropH    uint32 // Horizontal crop amount (left/right)
}

// NewVidIdx creates a new video index for the given file path.
func NewVidIdx(path string, showProgress bool) (*VidIdx, error) {
	Init()

	errInfo := C.create_error_info()
	defer C.free_error_info(errInfo)

	cPath := C.CString(path)
	defer C.free(unsafe.Pointer(cPath))

	// Create indexer
	indexer := C.FFMS_CreateIndexer(cPath, errInfo)
	if indexer == nil {
		return nil, fmt.Errorf("failed to create indexer: %s", C.GoString(C.get_error_message(errInfo)))
	}

	// Index all tracks
	C.FFMS_TrackIndexSettings(indexer, -1, 1, 0)

	// Run indexing
	idx := C.FFMS_DoIndexing2(indexer, C.int(0), errInfo)
	if idx == nil {
		return nil, fmt.Errorf("failed to index: %s", C.GoString(C.get_error_message(errInfo)))
	}

	return &VidIdx{ptr: idx, videoPath: path}, nil
}

// Close releases the index resources.
func (v *VidIdx) Close() {
	if v.ptr != nil {
		C.FFMS_DestroyIndex(v.ptr)
		v.ptr = nil
	}
}

// GetVidInf retrieves video information from the index.
func GetVidInf(idx *VidIdx) (*VidInf, error) {
	if idx == nil || idx.ptr == nil {
		return nil, fmt.Errorf("nil index")
	}

	errInfo := C.create_error_info()
	defer C.free_error_info(errInfo)

	// Get video track number
	trackNum := C.FFMS_GetFirstTrackOfType(idx.ptr, C.FFMS_TYPE_VIDEO, errInfo)
	if trackNum < 0 {
		return nil, fmt.Errorf("no video track found: %s", C.GoString(C.get_error_message(errInfo)))
	}

	cPath := C.CString(idx.videoPath)
	defer C.free(unsafe.Pointer(cPath))

	// Create video source to get properties
	src := C.FFMS_CreateVideoSource(cPath, C.int(trackNum), idx.ptr, 0, C.FFMS_SEEK_NORMAL, errInfo)
	if src == nil {
		return nil, fmt.Errorf("failed to create video source: %s", C.GoString(C.get_error_message(errInfo)))
	}
	defer C.FFMS_DestroyVideoSource(src)

	// Get video properties
	props := C.FFMS_GetVideoProperties(src)
	if props == nil {
		return nil, fmt.Errorf("failed to get video properties")
	}

	// Get first frame to determine pixel format
	frame := C.FFMS_GetFrame(src, 0, errInfo)
	if frame == nil {
		return nil, fmt.Errorf("failed to get first frame: %s", C.GoString(C.get_error_message(errInfo)))
	}

	inf := &VidInf{
		Width:       uint32(frame.EncodedWidth),
		Height:      uint32(frame.EncodedHeight),
		FPSNum:      uint32(props.FPSNumerator),
		FPSDen:      uint32(props.FPSDenominator),
		Frames:      int(props.NumFrames),
		PixelFormat: int(frame.ConvertedPixelFormat),
	}

	// Determine if 10-bit based on pixel format
	// Common 10-bit formats: AV_PIX_FMT_YUV420P10LE (62), YUV420P10BE (63)
	// 8-bit: AV_PIX_FMT_YUV420P (0)
	pixFmt := int(frame.ConvertedPixelFormat)
	inf.Is10Bit = pixFmt >= 62 && pixFmt <= 67 // 10-bit range

	// Extract color metadata if available
	if frame.ColorPrimaries > 0 {
		cp := int32(frame.ColorPrimaries)
		inf.ColorPrimaries = &cp
	}
	// Note: FFMS2 header has typo "TransferCharateristics" (missing 'i')
	if frame.TransferCharateristics > 0 {
		tc := int32(frame.TransferCharateristics)
		inf.TransferCharacteristics = &tc
	}
	if frame.ColorSpace > 0 {
		mc := int32(frame.ColorSpace)
		inf.MatrixCoefficients = &mc
	}

	return inf, nil
}

// GetDecodeStrat determines the optimal decoding strategy based on video properties.
func GetDecodeStrat(idx *VidIdx, inf *VidInf, cropH, cropV uint32) (DecodeStrat, *CropCalc, error) {
	hasCrop := cropH > 0 || cropV > 0

	// Calculate cropped dimensions
	newW := inf.Width - 2*cropH
	newH := inf.Height - 2*cropV

	// Determine if we need stride handling
	// For simplicity, assume packed formats don't need stride handling
	needsStride := false

	var strat DecodeStrat
	if inf.Is10Bit {
		if hasCrop {
			if needsStride {
				strat = B10CropStride
			} else {
				strat = B10CropFast
			}
		} else {
			if needsStride {
				strat = B10Stride
			} else {
				strat = B10Fast
			}
		}
	} else {
		if hasCrop {
			if needsStride {
				strat = B8CropStride
			} else {
				strat = B8CropFast
			}
		} else {
			if needsStride {
				strat = B8Stride
			} else {
				strat = B8Fast
			}
		}
	}

	var cropCalc *CropCalc
	if hasCrop {
		bytesPerPixel := 1
		if inf.Is10Bit {
			bytesPerPixel = 2
		}

		cropCalc = &CropCalc{
			NewW:     newW,
			NewH:     newH,
			YStride:  int(inf.Width) * bytesPerPixel,
			UVStride: int(inf.Width) * bytesPerPixel / 2,
			YStart:   int(cropV)*int(inf.Width)*bytesPerPixel + int(cropH)*bytesPerPixel,
			YLen:     int(newW) * bytesPerPixel,
			UVOff:    int(cropV/2)*int(inf.Width)*bytesPerPixel/2 + int(cropH)*bytesPerPixel/2,
			UVLen:    int(newW) * bytesPerPixel / 2,
			CropV:    cropV,
			CropH:    cropH,
		}
	}

	return strat, cropCalc, nil
}

// ThrVidSrc creates a threaded video source from an index.
func ThrVidSrc(idx *VidIdx, threads int) (*VidSrc, error) {
	if idx == nil || idx.ptr == nil {
		return nil, fmt.Errorf("nil index")
	}

	errInfo := C.create_error_info()
	defer C.free_error_info(errInfo)

	// Get video track number
	trackNum := C.FFMS_GetFirstTrackOfType(idx.ptr, C.FFMS_TYPE_VIDEO, errInfo)
	if trackNum < 0 {
		return nil, fmt.Errorf("no video track found: %s", C.GoString(C.get_error_message(errInfo)))
	}

	cPath := C.CString(idx.videoPath)
	defer C.free(unsafe.Pointer(cPath))

	// Create video source with threading
	src := C.FFMS_CreateVideoSource(cPath, C.int(trackNum), idx.ptr, C.int(threads), C.FFMS_SEEK_NORMAL, errInfo)
	if src == nil {
		return nil, fmt.Errorf("failed to create video source: %s", C.GoString(C.get_error_message(errInfo)))
	}

	return &VidSrc{ptr: src}, nil
}

// Close releases the video source resources.
func (v *VidSrc) Close() {
	if v.ptr != nil {
		C.FFMS_DestroyVideoSource(v.ptr)
		v.ptr = nil
	}
}

// ExtractFrame extracts a single frame from the video source.
// Output is always 10-bit YUV420 (16-bit little-endian per sample).
// 8-bit sources are converted to 10-bit by left-shifting by 2.
func ExtractFrame(src *VidSrc, frameIdx int, output []byte, inf *VidInf, strat DecodeStrat, cropCalc *CropCalc) error {
	if src == nil || src.ptr == nil {
		return fmt.Errorf("nil video source")
	}

	errInfo := C.create_error_info()
	defer C.free_error_info(errInfo)

	// Get the frame
	frame := C.FFMS_GetFrame(src.ptr, C.int(frameIdx), errInfo)
	if frame == nil {
		return fmt.Errorf("failed to get frame %d: %s", frameIdx, C.GoString(C.get_error_message(errInfo)))
	}

	// Extract data based on strategy
	width := inf.Width
	height := inf.Height
	if cropCalc != nil {
		width = cropCalc.NewW
		height = cropCalc.NewH
	}

	// Output is always 10-bit (16 bits per sample)
	yPlaneSize := int(width) * int(height) * 2       // Y: 2 bytes per pixel
	uPlaneSize := int(width) * int(height) / 4 * 2   // U: 1/4 pixels, 2 bytes each
	vPlaneSize := int(width) * int(height) / 4 * 2   // V: 1/4 pixels, 2 bytes each

	expectedSize := yPlaneSize + uPlaneSize + vPlaneSize
	if len(output) < expectedSize {
		return fmt.Errorf("output buffer too small: need %d, got %d", expectedSize, len(output))
	}

	// Get source data pointers
	yData := unsafe.Slice((*byte)(unsafe.Pointer(frame.Data[0])), int(frame.Linesize[0])*int(inf.Height))
	uData := unsafe.Slice((*byte)(unsafe.Pointer(frame.Data[1])), int(frame.Linesize[1])*int(inf.Height/2))
	vData := unsafe.Slice((*byte)(unsafe.Pointer(frame.Data[2])), int(frame.Linesize[2])*int(inf.Height/2))

	if inf.Is10Bit {
		// Source is 10-bit, copy directly
		srcYStride := int(frame.Linesize[0])
		srcUVStride := int(frame.Linesize[1])
		dstYStride := int(width) * 2
		dstUVStride := int(width/2) * 2

		// Copy Y plane
		copyPlane10bit(output[:yPlaneSize], yData, int(height), dstYStride, srcYStride)
		// Copy U plane
		copyPlane10bit(output[yPlaneSize:yPlaneSize+uPlaneSize], uData, int(height/2), dstUVStride, srcUVStride)
		// Copy V plane
		copyPlane10bit(output[yPlaneSize+uPlaneSize:], vData, int(height/2), dstUVStride, srcUVStride)
	} else {
		// Source is 8-bit, convert to 10-bit (left shift by 2)
		srcYStride := int(frame.Linesize[0])
		srcUVStride := int(frame.Linesize[1])

		// Convert Y plane
		convert8to10bit(output[:yPlaneSize], yData, int(width), int(height), srcYStride)
		// Convert U plane
		convert8to10bit(output[yPlaneSize:yPlaneSize+uPlaneSize], uData, int(width/2), int(height/2), srcUVStride)
		// Convert V plane
		convert8to10bit(output[yPlaneSize+uPlaneSize:], vData, int(width/2), int(height/2), srcUVStride)
	}

	return nil
}

// copyPlane10bit copies a 10-bit plane handling stride differences.
func copyPlane10bit(dst, src []byte, rows, dstStride, srcStride int) {
	srcOff := 0
	dstOff := 0
	for row := 0; row < rows; row++ {
		copy(dst[dstOff:dstOff+dstStride], src[srcOff:srcOff+dstStride])
		srcOff += srcStride
		dstOff += dstStride
	}
}

// convert8to10bit converts 8-bit YUV data to 10-bit by left-shifting by 2.
// Output is 16-bit little-endian per sample.
func convert8to10bit(dst, src []byte, width, height, srcStride int) {
	dstOff := 0
	for row := 0; row < height; row++ {
		srcRowStart := row * srcStride
		for col := 0; col < width; col++ {
			// Read 8-bit sample and convert to 10-bit (left shift by 2)
			sample8 := uint16(src[srcRowStart+col])
			sample10 := sample8 << 2

			// Write as 16-bit little-endian
			dst[dstOff] = byte(sample10 & 0xFF)
			dst[dstOff+1] = byte(sample10 >> 8)
			dstOff += 2
		}
	}
}

// copyPlaneCropped copies plane data with cropping.
func copyPlaneCropped(dst, src []byte, rows, startOffset, rowLen, stride int) {
	srcOff := startOffset
	dstOff := 0
	for row := 0; row < rows; row++ {
		copy(dst[dstOff:dstOff+rowLen], src[srcOff:srcOff+rowLen])
		srcOff += stride
		dstOff += rowLen
	}
}

// CalcPackedSize calculates the buffer size for 10-bit packed YUV420 format.
func CalcPackedSize(w, h uint32) int {
	// YUV420 10-bit: Y = w*h*2, U = w*h/4*2, V = w*h/4*2
	return int(w) * int(h) * 3 // 2 bytes per Y + 0.5 bytes per U + 0.5 bytes per V = 3 bytes total per pixel pair
}

// Calc8BitSize calculates the buffer size for 8-bit YUV420 format.
func Calc8BitSize(w, h uint32) int {
	// YUV420 8-bit: Y = w*h, U = w*h/4, V = w*h/4
	return int(w) * int(h) * 3 / 2
}

// CalcFrameSize returns the buffer size needed for a frame given video info.
// Always returns 10-bit size since we convert 8-bit sources to 10-bit for encoding.
func CalcFrameSize(inf *VidInf, cropCalc *CropCalc) int {
	w := inf.Width
	h := inf.Height
	if cropCalc != nil {
		w = cropCalc.NewW
		h = cropCalc.NewH
	}

	// Always use 10-bit size - 8-bit sources are converted to 10-bit
	return CalcPackedSize(w, h)
}
