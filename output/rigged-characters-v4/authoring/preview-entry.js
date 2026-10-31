import * as THREE from 'three';
import { GLTFLoader } from 'three/examples/jsm/loaders/GLTFLoader.js';
import { OrbitControls } from 'three/examples/jsm/controls/OrbitControls.js';
import { VRMLoaderPlugin } from '@pixiv/three-vrm';

const canvas = document.querySelector('canvas');
const status = document.querySelector('#status');
const selector = document.querySelector('#animation');
const renderer = new THREE.WebGLRenderer({canvas, antialias:true, alpha:false});
renderer.setPixelRatio(Math.min(devicePixelRatio, 2));
renderer.outputColorSpace = THREE.SRGBColorSpace;
renderer.toneMapping = THREE.ACESFilmicToneMapping;
renderer.toneMappingExposure = 1;
const scene = new THREE.Scene();
scene.background = new THREE.Color('#14121d');
const camera = new THREE.PerspectiveCamera(30, 1, .01, 50);
camera.position.set(1.55, 1.10, 2.8);
const controls = new OrbitControls(camera, canvas);
controls.target.set(0,.75,0);
controls.enableDamping = true;
controls.minDistance = .6;
controls.maxDistance = 8;
scene.add(new THREE.HemisphereLight(0xfff4f5,0x41415e,1.7));
for (const [x,y,z,color,power] of [[2,3,3,0xffeddd,2.2],[-2,2,1,0xc4d7ff,.7],[1,2,-2,0xffa6ca,2.0]]) {
  const light = new THREE.DirectionalLight(color,power); light.position.set(x,y,z);scene.add(light);
}
let mixer, model, action, skeleton, vrm;
let paused = false;
const raw = atob(window.MODEL_BASE64);
const bytes = Uint8Array.from(raw, c=>c.charCodeAt(0));
const loader = new GLTFLoader();
loader.register(parser=>new VRMLoaderPlugin(parser));
try {
  const gltf = await loader.parseAsync(bytes.buffer,'');
  vrm = gltf.userData.vrm;
  if (!vrm) throw new Error('VRM humanoid plugin did not recognize this model');
  vrm.humanoid.autoUpdateHumanBones = false;
  model = gltf.scene;
  scene.add(model);
  skeleton = new THREE.SkeletonHelper(model);
  skeleton.visible=false; scene.add(skeleton);
  let triangles=0,bones=0,skinned=0;
  model.traverse(node=>{
    if(node.isBone) bones++;
    if(node.isSkinnedMesh) skinned++;
    if(node.isMesh) triangles+=(node.geometry.index?.count??node.geometry.attributes.position.count)/3;
  });
  const clips = gltf.animations;
  mixer = new THREE.AnimationMixer(model);
  const bind = document.createElement('option'); bind.value='';bind.textContent='Bind pose (T-pose)';selector.append(bind);
  for(const clip of clips){const option=document.createElement('option');option.value=clip.name;option.textContent=clip.name.replaceAll('_',' ');selector.append(option);}
  selector.addEventListener('change',()=>{
    if(action){action.stop();action=null;}
    mixer.stopAllAction();
    model.traverse(node=>{if(node.isSkinnedMesh)node.skeleton.pose();});
    const clip=clips.find(c=>c.name===selector.value);
    if(clip){action=mixer.clipAction(clip);action.reset().play();}
  });
  selector.value='idle';selector.dispatchEvent(new Event('change'));
  status.textContent=`VRM loaded · ${Math.round(triangles).toLocaleString()} triangles · ${bones} joints · ${skinned} skinned meshes · ${clips.length} clips`;
  status.dataset.loaded='true';
  window.MODEL_QA={vrm:true,triangles,bones,skinned,clips:clips.map(c=>c.name)};
  document.querySelector('#pause').addEventListener('click',e=>{paused=!paused;e.target.textContent=paused?'Play animation':'Pause animation';});
  document.querySelector('#skeleton').addEventListener('change',e=>skeleton.visible=e.target.checked);
  document.querySelector('#front').addEventListener('click',()=>{camera.position.set(0,.95,3.5);controls.target.set(0,.75,0);});
  document.querySelector('#back').addEventListener('click',()=>{camera.position.set(0,.95,-3.5);controls.target.set(0,.75,0);});
  document.querySelector('#side').addEventListener('click',()=>{camera.position.set(3.5,.95,0);controls.target.set(0,.75,0);});
} catch(error) {
  status.textContent=`Model load failed: ${error.message}`;status.dataset.loaded='false';console.error(error);
}
function resize(){const w=canvas.clientWidth,h=canvas.clientHeight;renderer.setSize(w,h,false);camera.aspect=w/h;camera.updateProjectionMatrix();}
new ResizeObserver(resize).observe(canvas);
let last=performance.now(),elapsed=0;
function frame(now){requestAnimationFrame(frame);const dt=Math.min(.05,(now-last)/1000);last=now;if(!paused){elapsed+=dt;mixer?.update(dt);}if(vrm){const blink = document.querySelector('#blink').checked && elapsed%4>.0 && elapsed%4<.16 ? Math.sin((elapsed%4)/.16*Math.PI):0;vrm.expressionManager?.setValue('blink',blink);vrm.update(dt);}controls.update();renderer.render(scene,camera);}
requestAnimationFrame(frame);
