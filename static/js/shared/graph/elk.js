const ELK = globalThis.ELK;

if (!ELK) {
    throw new Error("ELK.js has not been loaded.");
}

const elk = new ELK({
    workerUrl: "/assets/js/vendor/elkjs/lib/elk-worker.min.js"
});

export default elk;