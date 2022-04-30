use basic::{base64::base64_decode, brokenboard::BrokenBoard};
use criterion::{criterion_group, criterion_main, Criterion};

fn criterion_benchmark(c: &mut Criterion) {
    let mut group = c.benchmark_group("supporting_queues");

    for board in [
        "E8______PA6DanAZIGlR_OET0wsMcXkDB.o",
        "E8______PQ7CI5Q_IQfxYOETExqMcWgJB.o",
        "E8_x2_BACQ7IqTRtO6HS7Yq.m",
        "E8______PAZEIFhZMIFSdS0tSyfGWFhFB.o",
        "E8______PQqD6nwIJABSMTQPDQgYe0wsE.s",
        "E8______PwYEGDxYMGDyYoGs1gxGs2g5G.s",
        "E8______PQrDqjQZKqpRdOqFSNT6nT1sq.s",
    ] {
        group.bench_with_input(board, board, |b, s| {
            b.iter(|| {
                let bb = BrokenBoard::decode(&base64_decode(s).unwrap()).unwrap();
                bb.supporting_queues()
            });
        });
    }

    group.finish();
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
