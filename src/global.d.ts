/// <reference types="svelte" />
const invoke = window.__TAURI__.invoke;
declare module "*.svelte" {
  const value: any; // Add better type definitions here if desired.
  export default value;
}