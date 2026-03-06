import { createFileRoute } from "@tanstack/react-router";
import { useEffect, useRef, useState } from "react";
import * as THREE from "three";
import { TextGeometry } from "three/addons/geometries/TextGeometry.js";
import { FontLoader } from "three/addons/loaders/FontLoader.js";

export const Route = createFileRoute("/welcome")({
  component: WelcomePage,
});

function WelcomePage() {
  const containerRef = useRef<HTMLDivElement>(null);
  const [showModal, setShowModal] = useState(false);

  useEffect(() => {
    const container = containerRef.current;
    if (!container) return;

    // Scene setup
    const scene = new THREE.Scene();

    // Nebula background — fullscreen shader quad using fbm noise.
    const nebulaVertexShader = `
      varying vec2 vUv;
      void main() {
        vUv = uv;
        gl_Position = vec4(position, 1.0);
      }
    `;
    const nebulaFragmentShader = `
      uniform float uTime;
      varying vec2 vUv;

      // Simplex-style hash
      vec3 hash3(vec3 p) {
        p = vec3(
          dot(p, vec3(127.1, 311.7, 74.7)),
          dot(p, vec3(269.5, 183.3, 246.1)),
          dot(p, vec3(113.5, 271.9, 124.6))
        );
        return -1.0 + 2.0 * fract(sin(p) * 43758.5453123);
      }

      // 3D gradient noise
      float noise(vec3 p) {
        vec3 i = floor(p);
        vec3 f = fract(p);
        vec3 u = f * f * (3.0 - 2.0 * f);

        return mix(mix(mix(dot(hash3(i + vec3(0,0,0)), f - vec3(0,0,0)),
                           dot(hash3(i + vec3(1,0,0)), f - vec3(1,0,0)), u.x),
                       mix(dot(hash3(i + vec3(0,1,0)), f - vec3(0,1,0)),
                           dot(hash3(i + vec3(1,1,0)), f - vec3(1,1,0)), u.x), u.y),
                   mix(mix(dot(hash3(i + vec3(0,0,1)), f - vec3(0,0,1)),
                           dot(hash3(i + vec3(1,0,1)), f - vec3(1,0,1)), u.x),
                       mix(dot(hash3(i + vec3(0,1,1)), f - vec3(0,1,1)),
                           dot(hash3(i + vec3(1,1,1)), f - vec3(1,1,1)), u.x), u.y), u.z);
      }

      // Fractal Brownian motion
      float fbm(vec3 p) {
        float value = 0.0;
        float amplitude = 0.5;
        for (int i = 0; i < 6; i++) {
          value += amplitude * noise(p);
          p *= 2.0;
          amplitude *= 0.5;
        }
        return value;
      }

      void main() {
        vec2 uv = vUv;
        float t = uTime * 0.02;

        // Sample position in 3D noise space
        vec3 p = vec3(uv * 3.0, t);

        // Layer multiple fbm calls for cloud-like structure
        float n1 = fbm(p);
        float n2 = fbm(p + vec3(5.2, 1.3, 2.8) + n1 * 0.5);
        float n3 = fbm(p + vec3(1.7, 9.2, 3.4) + n2 * 0.5);

        // Color channels — deep purple/blue nebula
        vec3 col = vec3(0.03, 0.03, 0.06); // dark base
        col += vec3(0.15, 0.04, 0.25) * smoothstep(-0.2, 0.6, n2); // purple clouds
        col += vec3(0.03, 0.10, 0.25) * smoothstep(-0.1, 0.5, n3); // blue wisps
        col += vec3(0.20, 0.05, 0.15) * smoothstep(0.1, 0.8, n1 * n2); // magenta highlights

        // Subtle vignette
        float vig = 1.0 - 0.4 * length(uv - 0.5);
        col *= vig;

        gl_FragColor = vec4(col, 1.0);
      }
    `;

    const nebulaUniforms = { uTime: { value: 0.0 } };
    const nebulaMaterial = new THREE.ShaderMaterial({
      vertexShader: nebulaVertexShader,
      fragmentShader: nebulaFragmentShader,
      uniforms: nebulaUniforms,
      depthWrite: false,
    });
    const nebulaQuad = new THREE.Mesh(new THREE.PlaneGeometry(2, 2), nebulaMaterial);
    nebulaQuad.frustumCulled = false;

    // Render nebula to a separate scene so it sits behind everything.
    const nebulaScene = new THREE.Scene();
    const nebulaCamera = new THREE.Camera();
    nebulaScene.add(nebulaQuad);

    scene.background = null;

    const camera = new THREE.PerspectiveCamera(
      50,
      container.clientWidth / container.clientHeight,
      0.1,
      1000,
    );
    // Pull camera back on narrow viewports so text always fits.
    const baseZ = 12;
    const baseAspect = 16 / 9;
    function getCameraZ() {
      const aspect = container!.clientWidth / container!.clientHeight;
      return aspect < baseAspect ? baseZ * (baseAspect / aspect) : baseZ;
    }
    camera.position.z = getCameraZ();

    const renderer = new THREE.WebGLRenderer({ antialias: true });
    renderer.setSize(container.clientWidth, container.clientHeight);
    renderer.setPixelRatio(window.devicePixelRatio);
    container.appendChild(renderer.domElement);

    // Lights
    const ambient = new THREE.AmbientLight(0x808080, 1.5);
    scene.add(ambient);

    const light1 = new THREE.PointLight(0x00ffff, 80, 50);
    light1.position.set(5, 5, 8);
    scene.add(light1);

    const light2 = new THREE.PointLight(0xff00ff, 80, 50);
    light2.position.set(-5, -3, 8);
    scene.add(light2);

    const light3 = new THREE.PointLight(0xffff00, 40, 50);
    light3.position.set(0, 8, 5);
    scene.add(light3);

    // Load font and create text
    const loader = new FontLoader();
    let internetMesh: THREE.Mesh | null = null;

    loader.load("/helvetiker_bold.typeface.json", (font) => {
      // "welcome to" — flat, above
      const welcomeGeo = new TextGeometry("WELCOME TO", {
        font,
        size: 0.4,
        depth: 0.05,
        curveSegments: 12,
      });
      welcomeGeo.computeBoundingBox();
      welcomeGeo.center();

      const welcomeMat = new THREE.MeshStandardMaterial({
        color: 0xcccccc,
        metalness: 0.3,
        roughness: 0.7,
      });
      const welcomeMesh = new THREE.Mesh(welcomeGeo, welcomeMat);
      welcomeMesh.position.y = 2.5;
      scene.add(welcomeMesh);

      // "THE INTERNET" — big, 3D, spinning
      const internetGeo = new TextGeometry("THE INTERNET", {
        font,
        size: 1.2,
        depth: 0.6,
        curveSegments: 16,
        bevelEnabled: true,
        bevelThickness: 0.08,
        bevelSize: 0.04,
        bevelSegments: 8,
      });
      internetGeo.computeBoundingBox();
      internetGeo.center();

      const internetMat = new THREE.MeshStandardMaterial({
        color: 0x66f5ff,
        metalness: 0.5,
        roughness: 0.3,
        emissive: 0x00aacc,
        emissiveIntensity: 0.3,
        envMapIntensity: 1.0,
      });
      internetMesh = new THREE.Mesh(internetGeo, internetMat);
      internetMesh.position.y = 0;
      scene.add(internetMesh);

      // Subtitle — small, below
      const subtitleGeo = new TextGeometry("ADVANCED COMMUNICATIONS NETWORK FOR PLANET EARTH", {
        font,
        size: 0.22,
        depth: 0.02,
        curveSegments: 12,
      });
      subtitleGeo.computeBoundingBox();
      subtitleGeo.center();

      const subtitleMat = new THREE.MeshStandardMaterial({
        color: 0xdddddd,
        metalness: 0.3,
        roughness: 0.7,
      });
      const subtitleMesh = new THREE.Mesh(subtitleGeo, subtitleMat);
      subtitleMesh.position.y = -2;
      scene.add(subtitleMesh);
    });

    // Stars
    const starsGeo = new THREE.BufferGeometry();
    const starPositions = new Float32Array(3000);
    for (let i = 0; i < 3000; i++) {
      starPositions[i] = (Math.random() - 0.5) * 100;
    }
    starsGeo.setAttribute("position", new THREE.BufferAttribute(starPositions, 3));
    const starsMat = new THREE.PointsMaterial({
      color: 0xffffff,
      size: 0.05,
    });
    const stars = new THREE.Points(starsGeo, starsMat);
    scene.add(stars);

    // Animation
    let animationId: number;
    const clock = new THREE.Clock();

    function animate() {
      animationId = requestAnimationFrame(animate);
      const t = clock.getElapsedTime();

      if (internetMesh) {
        internetMesh.rotation.y = Math.sin(t * 0.5) * 0.4;
        internetMesh.rotation.x = Math.sin(t * 0.3) * 0.1;
        internetMesh.position.y = Math.sin(t * 0.8) * 0.3;
      }

      // Slowly rotate lights
      light1.position.x = Math.sin(t * 0.7) * 8;
      light1.position.y = Math.cos(t * 0.5) * 5;
      light2.position.x = Math.cos(t * 0.3) * 8;
      light2.position.y = Math.sin(t * 0.4) * 5;

      stars.rotation.y = t * 0.02;
      stars.rotation.x = t * 0.01;

      // Update nebula time uniform
      nebulaUniforms.uTime.value = t;

      // Render nebula background first, then main scene on top
      renderer.autoClear = false;
      renderer.clear();
      renderer.render(nebulaScene, nebulaCamera);
      renderer.render(scene, camera);
      renderer.autoClear = true;
    }
    animate();

    // Resize handler
    function onResize() {
      if (!container) return;
      camera.aspect = container.clientWidth / container.clientHeight;
      camera.position.z = getCameraZ();
      camera.updateProjectionMatrix();
      renderer.setSize(container.clientWidth, container.clientHeight);
    }
    window.addEventListener("resize", onResize);

    return () => {
      cancelAnimationFrame(animationId);
      window.removeEventListener("resize", onResize);
      renderer.dispose();
      container.removeChild(renderer.domElement);
    };
  }, []);

  return (
    <div className="relative w-screen h-screen overflow-hidden">
      <div ref={containerRef} className="w-full h-full" />
      <button
        onClick={() => setShowModal(true)}
        className="absolute bottom-[12%] left-1/2 -translate-x-1/2 px-10 py-3.5 text-base font-sans tracking-widest uppercase text-white bg-accent/15 border border-accent/50 rounded cursor-pointer transition-all duration-300 hover:bg-accent/30 hover:border-accent/80 hover:shadow-[0_0_20px_rgba(125,207,255,0.3)]"
      >
        Begin Your Journey
      </button>

      {showModal && (
        <div
          className="absolute inset-0 bg-black/85 flex flex-col items-center justify-center z-10 font-sans"
          onClick={() => setShowModal(false)}
        >
          <div
            onClick={(e) => e.stopPropagation()}
            className="flex flex-col items-center gap-6 max-w-[640px] w-[90%]"
          >
            <iframe
              width="560"
              height="315"
              src="https://www.youtube.com/embed/dQw4w9WgXcQ?autoplay=1"
              title="Welcome to the Internet"
              allow="autoplay; encrypted-media"
              allowFullScreen
              className="rounded-lg border border-accent/30 max-w-full"
            />

            <p className="text-foreground-dark text-lg text-center leading-relaxed m-0">
              You have been Rick Rolled.
              <br />
              Everybody gets Rick Rolled.
              <br />
              <span className="text-accent">Welcome to the Internet.</span>
            </p>

            <div className="flex gap-4 flex-wrap justify-center">
              <a
                href="https://github.com/termsurf/termsurf"
                className="text-accent no-underline px-5 py-2 border border-accent/30 rounded text-sm font-sans tracking-wide transition-all duration-300 hover:bg-accent/15 hover:border-accent/60"
              >
                github.com
              </a>
              <a
                href="https://termsurf.com"
                className="text-accent no-underline px-5 py-2 border border-accent/30 rounded text-sm font-sans tracking-wide transition-all duration-300 hover:bg-accent/15 hover:border-accent/60"
              >
                termsurf.com
              </a>
            </div>
          </div>
        </div>
      )}
    </div>
  );
}
