import { JsCompiler } from '@unpack-js/binding';

const registry = new FinalizationRegistry(val => {
    console.log("finalized", val);
});

function main() {
    console.time("build");
    const compiler = new JsCompiler(import.meta.dirname, 'index.js', [
        {
            onResolve: (arg: string) => {
                console.log(`onResolve: ${arg}`);
                return arg;
            },
            onLoad: (arg: string) => {
                return Buffer.from("console.log('hello world')", 'utf-8');
            },
            thisCompilation: (arg: any) => {
                console.log(`thisCompilation: ${arg}`);
            },
        },
    ]);
    
    registry.register(compiler, 'compiler');
    compiler.build(() => {
        console.timeEnd("build");
        console.log("build done");
    });
    console.log("running other code");
}
main();

setTimeout(() => {
    if (global.gc) {
        console.log("triggering gc");
        gc();
        gc();
        gc();
    }
}, 2000);
