from setuptools import setup, find_packages

setup(
    name="drapto",
    version="0.1.0",
    packages=find_packages(),
    install_requires=[
        "ffmpeg-python",
        "rich>=13.0.0",  # Explicit minimum version
        "scenedetect[opencv]",
        "psutil",
    ],
    entry_points={
        "console_scripts": [
            "drapto=drapto.__main__:main",
        ],
    },
    python_requires=">=3.8",
)
