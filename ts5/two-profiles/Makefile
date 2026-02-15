.PHONY: all clean shaders

all: shaders
	swift build
	cp Sources/TwoProfiles/shaders.metallib .build/debug/shaders.metallib

shaders: Sources/TwoProfiles/shaders.metallib

Sources/TwoProfiles/shaders.metallib: Sources/TwoProfiles/Shaders.metal
	xcrun metal -c Sources/TwoProfiles/Shaders.metal -o Sources/TwoProfiles/Shaders.air
	xcrun metallib Sources/TwoProfiles/Shaders.air -o Sources/TwoProfiles/shaders.metallib
	rm Sources/TwoProfiles/Shaders.air

clean:
	swift package clean
	rm -f Sources/TwoProfiles/Shaders.air Sources/TwoProfiles/shaders.metallib
	rm -f .build/debug/shaders.metallib
