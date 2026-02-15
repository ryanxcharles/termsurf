.PHONY: all clean shaders

all: shaders
	swift build
	cp Sources/ThreeProfiles/shaders.metallib .build/debug/shaders.metallib

shaders: Sources/ThreeProfiles/shaders.metallib

Sources/ThreeProfiles/shaders.metallib: Sources/ThreeProfiles/Shaders.metal
	xcrun metal -c Sources/ThreeProfiles/Shaders.metal -o Sources/ThreeProfiles/Shaders.air
	xcrun metallib Sources/ThreeProfiles/Shaders.air -o Sources/ThreeProfiles/shaders.metallib
	rm Sources/ThreeProfiles/Shaders.air

clean:
	swift package clean
	rm -f Sources/ThreeProfiles/Shaders.air Sources/ThreeProfiles/shaders.metallib
	rm -f .build/debug/shaders.metallib
