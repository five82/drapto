#!/usr/bin/env bash

# Build ffmpeg with dynamic linking using Linux Homebrew

# Get the directory where the script is located
SCRIPT_DIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )" && pwd )"

# Parse command line arguments
ENABLE_LOGGING=false
for arg in "$@"; do
    case $arg in
        --log)
            ENABLE_LOGGING=true
            shift
            ;;
    esac
done

# Set up logging only if --log flag is passed
if [[ "$ENABLE_LOGGING" == "true" ]]; then
    LOG_FILE="${SCRIPT_DIR}/ffmpeg_build.log"
    exec 1> >(tee -a "${LOG_FILE}")
    exec 2>&1
    echo "Build started at $(date)"
    echo "======================="
fi

cleanup() {
    local exit_code=$?
    if [ $exit_code -ne 0 ]; then
        echo "Build failed with exit code: $exit_code"
        echo "Build directory ${BUILD_DIR} preserved for debugging"
    else
        echo "Build completed successfully"
    fi
}

trap cleanup EXIT

# Function to check if a command exists
command_exists() {
    command -v "$1" >/dev/null 2>&1
}

# Function to check Homebrew dependencies
check_brew_deps() {
    local missing_deps=()
    local required_pkgs=(
        # Build tools
        "autoconf"
        "automake"
        "libtool"
        "git"
        "gcc"
        "make"
        "ninja"
        "meson"
        "cmake"
        "nasm"
        "yasm"
        "pkg-config"
        
        # Libraries and their development files
        "expat"
        "gettext"
        "gperf"
        # Required libraries for FFmpeg
        "opus"
        "dav1d"
        "svt-av1"
    )

    echo "Checking Homebrew dependencies..."
    for pkg in "${required_pkgs[@]}"; do
        if ! brew list "$pkg" >/dev/null 2>&1; then
            missing_deps+=("$pkg")
        else
            echo "Found ${pkg}: $(brew list --versions ${pkg})"
        fi
    done

    if [ ${#missing_deps[@]} -ne 0 ]; then
        echo "Missing required packages. Installing them now..."
        brew install "${missing_deps[@]}"
    fi
}

# Update check_dependencies function
check_dependencies() {
    if ! command_exists brew; then
        echo "Error: Homebrew is not installed. Please install it first."
        echo "Visit https://brew.sh for installation instructions"
        exit 1
    fi
    
    # Set up environment to use Homebrew's tools and libraries
    eval "$(/home/linuxbrew/.linuxbrew/bin/brew shellenv)"
    
    check_brew_deps

    # Check for essential build tools
    local essential_tools=("git" "make" "pkg-config" "gcc")
    local missing_tools=()

    for tool in "${essential_tools[@]}"; do
        if ! command_exists "$tool"; then
            missing_tools+=("$tool")
        fi
    done

    if [ ${#missing_tools[@]} -ne 0 ]; then
        echo "Error: Missing essential build tools: ${missing_tools[*]}"
        exit 1
    fi

    echo "All required dependencies are installed."
    echo "Using Homebrew from: $(brew --prefix)"
    echo "Using GCC: $(which gcc)"
    echo "GCC version: $(gcc --version | head -n1)"
}

# Add this function after the check_dependencies function
check_build_dependencies() {
    echo "Checking build dependencies..."
    local missing_deps=()
    
    # Check pkg-config files
    local required_pc=(
        "opus"
        "SvtAv1Enc"
        "dav1d"
    )

    for pc in "${required_pc[@]}"; do
        echo "Checking ${pc}..."
        if ! PKG_CONFIG_PATH="${INSTALL_DIR}/lib/pkgconfig" pkg-config --exists "${pc}"; then
            missing_deps+=("${pc}")
            echo "  Not found!"
        else
            echo "  Found: $(PKG_CONFIG_PATH="${INSTALL_DIR}/lib/pkgconfig" pkg-config --modversion "${pc}")"
            echo "  Libs: $(PKG_CONFIG_PATH="${INSTALL_DIR}/lib/pkgconfig" pkg-config --libs "${pc}")"
        fi
    done

    # Check static libraries
    local required_libs=(
        "libopus.a"
        "libSvtAv1Enc.a"
        "libdav1d.a"
    )

    for lib in "${required_libs[@]}"; do
        echo "Checking ${lib}..."
        if ! find "${INSTALL_DIR}" -name "${lib}" | grep -q .; then
            missing_deps+=("${lib}")
            echo "  Not found!"
        else
            echo "  Found: $(find "${INSTALL_DIR}" -name "${lib}")"
        fi
    done

    if [ ${#missing_deps[@]} -ne 0 ]; then
        echo "Error: Missing required dependencies:"
        printf '%s\n' "${missing_deps[@]}"
        exit 1
    fi

    echo "All build dependencies are present."
}

# Create and enter build directory
BUILD_DIR="/tmp/ffmpeg_build"
mkdir -p $BUILD_DIR

INSTALL_DIR="${HOME}/.local"
mkdir -p "$INSTALL_DIR"

# Check system dependencies before starting any builds
check_dependencies

build_libdovi() {
    if ! pkg-config --exists libdovi; then
        echo "libdovi not found via pkg-config; building libdovi using dovi_tool..."
        # Clone the dovi_tool repo if not already present (you can adjust the clone directory as needed)
        if [ ! -d "dovi_tool_repo" ]; then
            git clone --depth=1 https://github.com/quietvoid/dovi_tool.git dovi_tool_repo
        fi
        cd dovi_tool_repo/dolby_vision || { echo "Cannot enter dovi_tool/dolby_vision directory"; exit 1; }
        # Build and install libdovi using cargo-c (make sure cargo-c is installed: cargo install cargo-c)
        CARGO_INSTALL_ROOT="$HOME/.local" cargo cinstall install
        cd ../.. || exit 1
    else
        echo "libdovi found via pkg-config."
    fi
}

build_libdovi

cd $BUILD_DIR

# Add this function before the FFmpeg build section
verify_compiler() {
    echo "Verifying compiler installation..."
    if ! gcc -v &>/dev/null; then
        echo "Error: gcc is not working properly"
        return 1
    fi
    
    # Create and run a simple test program
    local test_file="/tmp/test.c"
    echo "int main() { return 0; }" > "$test_file"
    if ! gcc -o /tmp/test "$test_file"; then
        echo "Error: gcc cannot create executables"
        return 1
    fi
    
    rm -f /tmp/test /tmp/test.c
    echo "Compiler verification successful"
    return 0
}

# Add this function after verify_compiler()
configure_ffmpeg() {
    local brew_prefix=$(brew --prefix)
    
    # Set pkg-config to prioritize Homebrew
    export PKG_CONFIG_PATH="${brew_prefix}/lib/pkgconfig:${PKG_CONFIG_PATH}"
    
    # Set compiler environment
    export CC=/usr/bin/gcc
    export CXX=/usr/bin/g++
    export CFLAGS="-I${brew_prefix}/include -O3"
    export CXXFLAGS="-I${brew_prefix}/include -O3"
    export LDFLAGS="-L${brew_prefix}/lib -Wl,-rpath,${brew_prefix}/lib -Wl,-rpath,${HOME}/.local/lib"
    export LD_LIBRARY_PATH="${brew_prefix}/lib:${LD_LIBRARY_PATH}"
    
    # Print versions of key dependencies
    echo "Checking dependency versions..."
    echo "opus: $(PKG_CONFIG_PATH="${brew_prefix}/lib/pkgconfig" pkg-config --modversion opus)"
    echo "SVT-AV1: $(PKG_CONFIG_PATH="${brew_prefix}/lib/pkgconfig" pkg-config --modversion SvtAv1Enc)"
    echo "dav1d: $(PKG_CONFIG_PATH="${brew_prefix}/lib/pkgconfig" pkg-config --modversion dav1d)"
    
    ./configure \
        --prefix="${HOME}/.local" \
        --enable-gpl \
        --enable-version3 \
        --enable-shared \
        --disable-static \
        --disable-doc \
        --disable-debug \
        --disable-ffplay \
        --enable-ffprobe \
        --disable-network \
        --disable-libssh \
        --disable-libsmbclient \
        --disable-gnutls \
        --extra-cflags="${CFLAGS}" \
        --extra-cxxflags="${CXXFLAGS}" \
        --extra-ldflags="${LDFLAGS}" \
        --extra-libs="-lpthread -lm" \
        --pkg-config-flags="--static" \
        --enable-libdav1d \
        --enable-libopus \
        --enable-libsvtav1 \
        --enable-libdovi \
	--enable-protocol=file \
	--enable-protocol=crypto \
        --cc=/usr/bin/gcc
}

# Build FFmpeg
git clone --depth=1 https://github.com/FFmpeg/FFmpeg.git
cd FFmpeg

# Verify compiler before proceeding
if ! verify_compiler; then
    echo "Error: Compiler verification failed"
    exit 1
fi

# Add diagnostic information
echo "Checking build environment..."
echo "gcc version: $(gcc -dumpversion)"
echo "Current directory: $(pwd)"
echo "PKG_CONFIG_PATH: ${PKG_CONFIG_PATH}"
echo "Install directory: ${INSTALL_DIR}"

# Clean any previous build artifacts if they exist
if [ -f "Makefile" ]; then
    make clean || true
    make distclean || true
fi

# Configure FFmpeg with platform-specific settings
if ! configure_ffmpeg; then
    echo "FFmpeg configure failed"
    exit 1
fi

# Verify configure succeeded
if [ ! -f "ffbuild/config.mak" ]; then
    echo "Error: FFmpeg configure did not create config.mak"
    exit 1
fi

# Use number of CPU cores or fallback to 2 if nproc is not available
NPROC=$(nproc 2>/dev/null || echo 2)
echo "Building with $NPROC parallel jobs..."

# Build FFmpeg
make -j"${NPROC}" || {
    echo "FFmpeg make failed"
    exit 1
}

make install || {
    echo "FFmpeg installation failed"
    exit 1
}

# Add this after the make install command, before the validation section
echo "Verifying FFmpeg build..."
if ! "${INSTALL_DIR}/bin/ffmpeg" -version &> /dev/null; then
    echo "Error: FFmpeg binary test failed"
    exit 1
fi

# Check library linkage
echo "Checking library dependencies..."
if ! ldd "${INSTALL_DIR}/bin/ffmpeg" | grep -q libavutil; then
    echo "Error: libavutil not properly linked"
    exit 1
fi

# Update validation section
echo "Validating FFmpeg installation..."

# Check if ffmpeg and ffprobe are in our local installation
if ! "${INSTALL_DIR}/bin/ffmpeg" -version &> /dev/null; then
    echo "Error: ffmpeg binary failed to execute"
    exit 1
fi

if ! "${INSTALL_DIR}/bin/ffprobe" -version &> /dev/null; then
    echo "Error: ffprobe binary failed to execute"
    exit 1
fi

echo "FFmpeg validation completed!"
echo "Build completed successfully!"
echo "Binaries location:"
echo "  ffmpeg:  ${INSTALL_DIR}/bin/ffmpeg"
echo "  ffprobe: ${INSTALL_DIR}/bin/ffprobe"


# Print library versions
echo "Checking library versions..."
echo "SVT-AV1:"
pkg-config --modversion SvtAv1Enc
pkg-config --libs SvtAv1Enc

echo "Opus:"
pkg-config --modversion opus
pkg-config --libs opus

echo "dav1d:"
pkg-config --modversion dav1d
pkg-config --libs dav1d

echo "FFmpeg configuration:"
"${INSTALL_DIR}/bin/ffmpeg" -version | grep configuration

# Clean up build directory after successful installation
echo "Cleaning up build directory..."
cd "${SCRIPT_DIR}"
if [ -d "${BUILD_DIR}" ]; then
    rm -rf "${BUILD_DIR}"
    echo "Build directory cleaned up"
else
    echo "No build directory to clean"
fi
