// Only measure the native WebView and capture pointers; desk coordinates live in Rust.
const workspace = document.getElementById("workspace");
const reportSize = () => dioxus.send([workspace.clientWidth, workspace.clientHeight]);
const observer = new ResizeObserver(reportSize);
observer.observe(workspace);
reportSize();
workspace.addEventListener("pointerdown", event => {
    if (event.button === 0 && !event.target.closest("button, input, textarea, a, .image-content")) {
        workspace.setPointerCapture(event.pointerId);
    }
});
await new Promise(() => {});
