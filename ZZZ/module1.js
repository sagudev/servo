import { Ass } from './module2.js';

(async () => {
    console.log("TLA");
    const ass = new Ass();
    console.log("1");
    let a = ass.ma();
    console.log("2");
    console.log(`PASS ${a}`);
    console.log("22");
    const res = await ass.mess("lol");
    console.log("3");
    console.log(`PASS ${res}`);
})();
console.log(`WTF`);
// no tla
console.log("no TLA");
const ass = new Ass();
console.log("no 1");
let a = ass.ma();
console.log("no 2");
console.log(`no PASS ${a}`);
console.log("no 22");