importScripts("./pkg/gomen.js");

function progress(piece_count, stage, board_idx, board_total) {
    let stage_progress = board_idx / board_total;
    let total_progress = (stage + stage_progress) / (2 + 2 * piece_count);

    postMessage({ kind: "progress", amount: total_progress });
}

async function main() {
    let legal_boards;

    let response = await fetch("./legal-boards.leb128");
    if (response.ok) {
        legal_boards = new Uint8Array(await response.arrayBuffer());
    } else {
        console.log("couldn't load legal boards");
    }

    await wasm_bindgen("./pkg/gomen_bg.wasm");
    let solver = new wasm_bindgen.Solver(legal_boards);

    console.log("ready");
    postMessage({ kind: "ready" });

    onmessage = message => {
        let query = message.data;

        if (solver.is_fast(query.garbage)) {
            postMessage({ kind: "fast", query });
        } else {
            postMessage({ kind: "slow", query });
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

        let solutions =
            solver.solve(queue, query.garbage, query.hold, query.physics)
                  .split(",");

        if (solutions[0] == "") {
            solutions = [];
        }

        postMessage({ kind: "ok", query, solutions });
    }
}
main();
