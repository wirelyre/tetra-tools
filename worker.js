importScripts("./pkg/solver.js");

function progress(piece_count, stage, board_idx, board_total) {
    let stage_progress = board_idx / board_total;
    let total_progress = (stage + stage_progress) / (2 + 2 * piece_count);

    postMessage({ kind: "progress", amount: total_progress });
}

async function main() {
    await wasm_bindgen("./pkg/solver_bg.wasm");
    let solver = new wasm_bindgen.Solver();

    console.log("ready");
    postMessage({ kind: "ready" });

    onmessage = message => {
        let query = message.data;

        if (!solver.possible(query.garbage)) {
            postMessage({ kind: "impossible", query });
            return;
        } else {
            postMessage({ kind: "possible", query });
        }

        let queue = new wasm_bindgen.Queue();

        let bag = /[ILJOSTZ]|\[([ILJOSTZ]+)\](\d*)|(\*)(\d*)/g;
        for (let match of query.queue.matchAll(bag)) {
            if (match[1]) {
                let count = parseInt(match[2], 10) || 1;
                queue.add_bag(match[1], count);
            } else if (match[3]) {
                let count = parseInt(match[4], 10) || 1;
                queue.add_bag("IJLOSTZ", count);
            } else {
                queue.add_shape(match[0]);
            }
        }

        let solutions = solver.solve(queue, query.garbage, query.hold).split(",");

        if (solutions[0] == "") {
            solutions = [];
        }

        postMessage({ kind: "ok", query, solutions });
    }
}
main();
