importScripts("./pkg/solver.js");

onmessage = message => {
    onmessage = null;

    if (message.data.kind == "start") {

        (async () => {

            console.time("init");

            let module = await WebAssembly.compileStreaming(fetch("./pkg/solver_bg.wasm"));

            let wasm = await wasm_bindgen(module);
            let init_ptr = wasm_bindgen.init();
            let memory = wasm.memory.buffer.slice();

            let subworker = new SubWorker(module, memory, init_ptr);
            onmessage = message => run(subworker, message.data);

            postMessage({
                id: message.data.id,
                success: true,
                kind: "ready",
            });

        })();

    } else if (message.data.kind == "start-subworker") {

        let { module, memory, init_ptr } = message.data;

        (async () => {

            console.time("boot-subworker");

            let wasm = await wasm_bindgen(module);

            wasm.memory.grow((memory.byteLength - wasm.memory.buffer.byteLength) / 64 / 1024);
            let bytes = new Uint8Array(wasm.memory.buffer);
            bytes.set(memory);

            let solver = wasm_bindgen.boot_solver(init_ptr);

            console.timeEnd("boot-subworker");

            console.log(solver, solver.possible(1n));

        })();

    }
}

class SubWorker {
    constructor(module, memory, init_ptr) {
        this._module = module;
        this._memory = memory;
        this._init_ptr = init_ptr;

        this.spawn();
    }

    spawn() {
        this.worker = new Worker("worker.js");
        this.worker.postMessage({
            kind: "start-subworker",
            module: this._module,
            memory: this._memory,
            init_ptr: this._init_ptr,
        });

        this.status = "READY";
    }

    call(message) {
        if (this.status == "BUSY") {
            this.worker.terminate();
            this._reject();

            this.spawn();
        }

        return new Promise((resolve, reject) => {
            this.worker.onmessage = resolve;
            this._reject = reject;
            this.worker.postMessage(message);
        });
    }
}

function run(subworker, message) {

    console.log(subworker, message);

}
