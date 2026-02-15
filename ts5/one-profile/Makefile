.PHONY: all clean shaders

all: shaders
	swift build
	cp Sources/OneProfile/shaders.metallib .build/debug/shaders.metallib

shaders: Sources/OneProfile/shaders.metallib

Sources/OneProfile/shaders.metallib: Sources/OneProfile/Shaders.metal
	xcrun metal -c Sources/OneProfile/Shaders.metal -o Sources/OneProfile/Shaders.air
	xcrun metallib Sources/OneProfile/Shaders.air -o Sources/OneProfile/shaders.metallib
	rm Sources/OneProfile/Shaders.air

clean:
	swift package clean
	rm -f Sources/OneProfile/Shaders.air Sources/OneProfile/shaders.metallib
	rm -f .build/debug/shaders.metallib
