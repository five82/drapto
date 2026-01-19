// Package vship provides CGO bindings to libvship for GPU-accelerated SSIMULACRA2.
package vship

/*
#cgo CFLAGS: -I/usr/local/include
#cgo LDFLAGS: -L/usr/local/lib -lvship

#include <stdlib.h>
#include <VshipAPI.h>
#include <VshipColor.h>
*/
import "C"
import (
	"errors"
	"fmt"
	"unsafe"
)

// Processor wraps a VSHIP SSIMULACRA2 handler for GPU-accelerated metric computation.
type Processor struct {
	handler C.Vship_SSIMU2Handler
}

// Version contains VSHIP library version information.
type Version struct {
	Major   int
	Minor   int
	Patch   int
	Backend string // "CUDA" or "HIP"
}

// GetVersion returns the VSHIP library version.
func GetVersion() Version {
	v := C.Vship_GetVersion()
	backend := "HIP"
	if v.backend == C.Vship_Cuda {
		backend = "CUDA"
	}
	return Version{
		Major:   int(v.major),
		Minor:   int(v.minor),
		Patch:   int(v.minorMinor),
		Backend: backend,
	}
}

// GetDetailedError returns detailed error information from the last error.
func GetDetailedError() string {
	buf := make([]C.char, 2048)
	C.Vship_GetDetailedLastError(&buf[0], 2048)
	return C.GoString(&buf[0])
}

// GetDeviceCount returns the number of available GPU devices.
func GetDeviceCount() (int, error) {
	var count C.int
	ret := C.Vship_GetDeviceCount(&count)
	if ret != C.Vship_NoError {
		detail := GetDetailedError()
		return 0, fmt.Errorf("failed to get device count: %s (detail: %s)", getErrorMessage(ret), detail)
	}
	return int(count), nil
}

// CheckGPU runs a full GPU check for the specified device.
func CheckGPU(gpuID int) error {
	ret := C.Vship_GPUFullCheck(C.int(gpuID))
	if ret != C.Vship_NoError {
		return fmt.Errorf("GPU check failed: %s", getErrorMessage(ret))
	}
	return nil
}

// InitDevice initializes the VSHIP GPU device.
// Must be called before creating any Processor instances.
func InitDevice() error {
	// Set device directly (matching xav's approach - skip device count check)
	ret := C.Vship_SetDevice(0)
	if ret != C.Vship_NoError {
		ver := GetVersion()
		detail := GetDetailedError()
		return fmt.Errorf("VSHIP %d.%d.%d (%s) failed to set device: %s (detail: %s)",
			ver.Major, ver.Minor, ver.Patch, ver.Backend, getErrorMessage(ret), detail)
	}
	return nil
}

// NewProcessor creates a new SSIMULACRA2 processor for the given video dimensions.
// Note: FFMS2 always decodes to 10-bit YUV, so the processor is always configured for 10-bit.
func NewProcessor(
	width, height uint32,
	matrix, transfer, primaries *int,
	colorRange, chromaSamplePos *int,
) (*Processor, error) {
	// Always 10-bit: FFMS2 converts 8-bit sources to 10-bit, and decodes probes as 10-bit
	srcCS := createColorspace(width, height, true, matrix, transfer, primaries, colorRange, chromaSamplePos)
	disCS := createColorspace(width, height, true, matrix, transfer, primaries, colorRange, chromaSamplePos)

	var handler C.Vship_SSIMU2Handler
	ret := C.Vship_SSIMU2Init(&handler, srcCS, disCS)
	if ret != C.Vship_NoError {
		return nil, fmt.Errorf("failed to initialize SSIMULACRA2: %s", getErrorMessage(ret))
	}

	return &Processor{handler: handler}, nil
}

// ComputeSSIMULACRA2 computes the SSIMULACRA2 score between source and distorted frames.
// srcPlanes and disPlanes are YUV plane pointers [Y, U, V].
// srcStrides and disStrides are line sizes for each plane.
func (p *Processor) ComputeSSIMULACRA2(
	srcPlanes, disPlanes [3]unsafe.Pointer,
	srcStrides, disStrides [3]int64,
) (float64, error) {
	// Allocate C arrays for pointers (CGO doesn't allow Go pointers containing Go pointers)
	srcPtrs := C.malloc(3 * C.size_t(unsafe.Sizeof(uintptr(0))))
	disPtrs := C.malloc(3 * C.size_t(unsafe.Sizeof(uintptr(0))))
	srcLineSizes := C.malloc(3 * C.size_t(unsafe.Sizeof(C.int64_t(0))))
	disLineSizes := C.malloc(3 * C.size_t(unsafe.Sizeof(C.int64_t(0))))
	defer C.free(srcPtrs)
	defer C.free(disPtrs)
	defer C.free(srcLineSizes)
	defer C.free(disLineSizes)

	// Fill the C arrays
	srcPtrSlice := (*[3]*C.uint8_t)(srcPtrs)
	disPtrSlice := (*[3]*C.uint8_t)(disPtrs)
	srcLineSlice := (*[3]C.int64_t)(srcLineSizes)
	disLineSlice := (*[3]C.int64_t)(disLineSizes)

	for i := range 3 {
		srcPtrSlice[i] = (*C.uint8_t)(srcPlanes[i])
		disPtrSlice[i] = (*C.uint8_t)(disPlanes[i])
		srcLineSlice[i] = C.int64_t(srcStrides[i])
		disLineSlice[i] = C.int64_t(disStrides[i])
	}

	var score C.double
	ret := C.Vship_ComputeSSIMU2(
		p.handler,
		&score,
		(**C.uint8_t)(srcPtrs),
		(**C.uint8_t)(disPtrs),
		(*C.int64_t)(srcLineSizes),
		(*C.int64_t)(disLineSizes),
	)

	if ret != C.Vship_NoError {
		return 0, fmt.Errorf("SSIMULACRA2 computation failed: %s", getErrorMessage(ret))
	}

	return float64(score), nil
}

// Close releases the VSHIP resources.
func (p *Processor) Close() error {
	if p.handler.id == 0 {
		return nil
	}
	ret := C.Vship_SSIMU2Free(p.handler)
	if ret != C.Vship_NoError {
		return errors.New("failed to free SSIMULACRA2 handler")
	}
	p.handler.id = 0
	return nil
}

// createColorspace creates a Vship_Colorspace_t struct from Go parameters.
func createColorspace(
	width, height uint32,
	is10Bit bool,
	matrix, transfer, primaries *int,
	colorRange, chromaSamplePos *int,
) C.Vship_Colorspace_t {
	var cs C.Vship_Colorspace_t

	cs.width = C.int64_t(width)
	cs.height = C.int64_t(height)
	cs.target_width = -1
	cs.target_height = -1

	// Sample type
	if is10Bit {
		cs.sample = C.Vship_SampleUINT10
	} else {
		cs.sample = C.Vship_SampleUINT8
	}

	// Color range
	cs._range = C.Vship_RangeLimited
	if colorRange != nil && *colorRange == 2 {
		cs._range = C.Vship_RangeFull
	}

	// Subsampling (4:2:0)
	cs.subsampling = C.Vship_ChromaSubsample_t{subw: 1, subh: 1}

	// Chroma location
	cs.chromaLocation = C.Vship_ChromaLoc_Left
	if chromaSamplePos != nil && *chromaSamplePos == 2 {
		cs.chromaLocation = C.Vship_ChromaLoc_TopLeft
	}

	// Color family
	cs.colorFamily = C.Vship_ColorYUV

	// YUV matrix
	cs.YUVMatrix = C.Vship_MATRIX_BT709
	if matrix != nil {
		switch *matrix {
		case 0:
			cs.YUVMatrix = C.Vship_MATRIX_RGB
		case 5:
			cs.YUVMatrix = C.Vship_MATRIX_BT470_BG
		case 6:
			cs.YUVMatrix = C.Vship_MATRIX_ST170_M
		case 9:
			cs.YUVMatrix = C.Vship_MATRIX_BT2020_NCL
		case 10:
			cs.YUVMatrix = C.Vship_MATRIX_BT2020_CL
		case 14:
			cs.YUVMatrix = C.Vship_MATRIX_BT2100_ICTCP
		}
	}

	// Transfer function
	cs.transferFunction = C.Vship_TRC_BT709
	if transfer != nil {
		switch *transfer {
		case 4:
			cs.transferFunction = C.Vship_TRC_BT470_M
		case 5:
			cs.transferFunction = C.Vship_TRC_BT470_BG
		case 6:
			cs.transferFunction = C.Vship_TRC_BT601
		case 8:
			cs.transferFunction = C.Vship_TRC_Linear
		case 13:
			cs.transferFunction = C.Vship_TRC_sRGB
		case 16:
			cs.transferFunction = C.Vship_TRC_PQ
		case 17:
			cs.transferFunction = C.Vship_TRC_ST428
		case 18:
			cs.transferFunction = C.Vship_TRC_HLG
		}
	}

	// Primaries
	cs.primaries = C.Vship_PRIMARIES_BT709
	if primaries != nil {
		switch *primaries {
		case -1:
			cs.primaries = C.Vship_PRIMARIES_INTERNAL
		case 4:
			cs.primaries = C.Vship_PRIMARIES_BT470_M
		case 5:
			cs.primaries = C.Vship_PRIMARIES_BT470_BG
		case 9:
			cs.primaries = C.Vship_PRIMARIES_BT2020
		}
	}

	// Crop (no cropping)
	cs.crop = C.Vship_CropRectangle_t{top: 0, bottom: 0, left: 0, right: 0}

	return cs
}

// getErrorMessage retrieves the error message for a VSHIP exception.
func getErrorMessage(exc C.Vship_Exception) string {
	buf := make([]C.char, 1024)
	C.Vship_GetErrorMessage(exc, &buf[0], 1024)
	return C.GoString(&buf[0])
}
