from setuptools import setup, find_packages

setup(
    name="drapto",
    version="0.1.0",
    packages=find_packages(),
    install_requires=[
        "ffmpeg-python",
        "rich",
        "scenedetect[opencv]",
        "psutil",
        "ray==2.42.1",
    ],
    entry_points={
        "console_scripts": [
            "drapto=drapto.__main__:main",
        ],
    },
    python_requires=">=3.8",
)
