/**
 * 3D Box Viewer using Three.js
 *
 * Renders a DVD case style box with front and back textures that can be rotated with mouse.
 */

import * as THREE from 'three';
import { OrbitControls } from 'three/addons/controls/OrbitControls.js';

console.log('Box3D: Three.js loaded successfully, version:', THREE.REVISION);

// DVD case proportions (width : height : depth)
const BOX_WIDTH = 1.35;
const BOX_HEIGHT = 1.9;
const BOX_DEPTH = 0.15;

// Active viewers map
const viewers = new Map();

/**
 * Initialize a 3D box viewer in a canvas element
 *
 * @param {string} canvasId - The ID of the canvas element
 * @param {string} frontUrl - URL for the front cover image
 * @param {string|null} backUrl - URL for the back cover image (optional)
 * @returns {object} - Viewer control object with destroy() method
 */
export function initBox3DViewer(canvasId, frontUrl, backUrl = null) {
    console.log(`Box3D: initBox3DViewer called with canvasId=${canvasId}`);

    try {
        // Clean up existing viewer if any
        if (viewers.has(canvasId)) {
            viewers.get(canvasId).destroy();
        }

        const canvas = document.getElementById(canvasId);
        if (!canvas) {
            console.error(`Box3D: Canvas element not found: ${canvasId}`);
            return null;
        }

        // Get dimensions from parent if canvas has no size yet
        const parent = canvas.parentElement;
        const grandparent = parent?.parentElement;
        const width = canvas.clientWidth || parent?.clientWidth || grandparent?.clientWidth || 400;
        const height = canvas.clientHeight || parent?.clientHeight || grandparent?.clientHeight || 350;

        console.log(`Box3D: Canvas found, dimensions: canvas=${canvas.clientWidth}x${canvas.clientHeight}, parent=${parent?.clientWidth}x${parent?.clientHeight}`);
        console.log(`Box3D: Initializing viewer ${canvasId} at ${width}x${height}`);
        console.log(`Box3D: Front URL: ${frontUrl}`);
        console.log(`Box3D: Back URL: ${backUrl}`);

    // Create scene
    const scene = new THREE.Scene();
    scene.background = new THREE.Color(0x1a1a2e);

    // Create camera
    const camera = new THREE.PerspectiveCamera(45, width / height, 0.1, 1000);
    camera.position.set(0, 0, 4);

    // Create renderer
    const renderer = new THREE.WebGLRenderer({
        canvas,
        antialias: true,
        alpha: true,
    });
    renderer.setSize(width, height);
    renderer.setPixelRatio(Math.min(window.devicePixelRatio, 2));

    // Create OrbitControls
    const controls = new OrbitControls(camera, renderer.domElement);
    controls.enableDamping = true;
    controls.dampingFactor = 0.05;
    controls.enableZoom = true;
    controls.minDistance = 2;
    controls.maxDistance = 8;
    controls.autoRotate = true;
    controls.autoRotateSpeed = 1.5;

    // Create texture loader
    const textureLoader = new THREE.TextureLoader();
    textureLoader.crossOrigin = 'anonymous';

    // Create box geometry
    const geometry = new THREE.BoxGeometry(BOX_WIDTH, BOX_HEIGHT, BOX_DEPTH);

    // Create materials array for each face [right, left, top, bottom, front, back]
    const materials = [];

    // Load front texture
    const frontTexture = textureLoader.load(frontUrl,
        () => {
            console.log('Box3D: Front texture loaded successfully');
            renderer.render(scene, camera);
        },
        undefined,
        (err) => console.error('Box3D: Failed to load front texture:', frontUrl, err)
    );

    // Load back texture if provided
    let backTexture = null;
    if (backUrl) {
        backTexture = textureLoader.load(backUrl,
            () => {
                console.log('Box3D: Back texture loaded successfully');
                renderer.render(scene, camera);
            },
            undefined,
            (err) => console.error('Box3D: Failed to load back texture:', backUrl, err)
        );
    }

    // Create materials for each face
    const spineMaterial = new THREE.MeshStandardMaterial({
        color: 0x222222,
        roughness: 0.8,
        metalness: 0.2,
    });

    const edgeMaterial = new THREE.MeshStandardMaterial({
        color: 0x333333,
        roughness: 0.7,
        metalness: 0.1,
    });

    // Face order: +X (right), -X (left), +Y (top), -Y (bottom), +Z (front), -Z (back)
    materials.push(spineMaterial); // Right side (spine)
    materials.push(spineMaterial); // Left side
    materials.push(edgeMaterial);  // Top
    materials.push(edgeMaterial);  // Bottom
    materials.push(new THREE.MeshStandardMaterial({ // Front cover
        map: frontTexture,
        roughness: 0.5,
        metalness: 0.1,
    }));
    materials.push(new THREE.MeshStandardMaterial({ // Back cover
        map: backTexture || frontTexture, // Use front as fallback
        roughness: 0.5,
        metalness: 0.1,
    }));

    // Create mesh
    const box = new THREE.Mesh(geometry, materials);
    scene.add(box);

    // Add lighting
    const ambientLight = new THREE.AmbientLight(0xffffff, 0.5);
    scene.add(ambientLight);

    const directionalLight = new THREE.DirectionalLight(0xffffff, 0.8);
    directionalLight.position.set(5, 5, 5);
    scene.add(directionalLight);

    const backLight = new THREE.DirectionalLight(0xffffff, 0.3);
    backLight.position.set(-5, -5, -5);
    scene.add(backLight);

    // Animation loop
    let animationId = null;
    let isDestroyed = false;

    function animate() {
        if (isDestroyed) return;
        animationId = requestAnimationFrame(animate);
        controls.update();
        renderer.render(scene, camera);
    }

    // Handle resize
    function handleResize() {
        if (isDestroyed) return;
        const newWidth = canvas.clientWidth;
        const newHeight = canvas.clientHeight;
        if (newWidth !== width || newHeight !== height) {
            camera.aspect = newWidth / newHeight;
            camera.updateProjectionMatrix();
            renderer.setSize(newWidth, newHeight);
        }
    }

    // Create resize observer
    const resizeObserver = new ResizeObserver(handleResize);
    resizeObserver.observe(canvas);

    // Start animation
    animate();

    // Create control object
    const viewer = {
        destroy() {
            isDestroyed = true;
            if (animationId) {
                cancelAnimationFrame(animationId);
            }
            resizeObserver.disconnect();
            controls.dispose();
            geometry.dispose();
            materials.forEach(m => {
                if (m.map) m.map.dispose();
                m.dispose();
            });
            renderer.dispose();
            viewers.delete(canvasId);
        },

        // Stop auto-rotation
        stopAutoRotate() {
            controls.autoRotate = false;
        },

        // Start auto-rotation
        startAutoRotate() {
            controls.autoRotate = true;
        },

        // Reset camera position
        resetCamera() {
            camera.position.set(0, 0, 4);
            controls.reset();
        },
    };

    viewers.set(canvasId, viewer);
    console.log(`Box3D: Viewer created successfully for ${canvasId}`);
    return viewer;
    } catch (error) {
        console.error(`Box3D: Error initializing viewer:`, error);
        return null;
    }
}

/**
 * Destroy a 3D box viewer
 *
 * @param {string} canvasId - The ID of the canvas element
 */
export function destroyBox3DViewer(canvasId) {
    if (viewers.has(canvasId)) {
        viewers.get(canvasId).destroy();
    }
}

// Expose to global scope for wasm_bindgen interop
window.Box3DViewer = {
    init: initBox3DViewer,
    destroy: destroyBox3DViewer,
};

console.log('Box3D: Module loaded, window.Box3DViewer available');
