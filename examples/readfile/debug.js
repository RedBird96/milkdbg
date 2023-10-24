async function f() {
    await addBreakpoint("CreateFileW");

    await init("./examples/readfile/main.exe");

    const call = await currentStackFrame();
    await print(call);

    var instr = await getCurrentInstructionString();
    await print(instr);

    await step();

    var instr = await getCurrentInstructionString();
    await print(instr);
}
f();