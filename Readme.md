# MilkDbg

Experimental Javascript powered Windows Debugger.
Only works with 32 bits executables at the moment.

## Example 

```js
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
```

If you run the example above you will get:

```
{"name":"CreateFileW","args":{"lpFileName":"\\\\?\\C:\\github\\milkdbg\\examples\\readfile\\main.rs","dwDesiredAccess":2147483648,"dwShareMode":7,"lpSecurityAttributes":0,"dwCreationDisposition":3,"dwFlagsAndAttributes":0,"hTemplateFile":0}}
```

Which is a descriptive object with all the known details from where the application is, given it just hit the breakpoint.

This makes super easy to debug an application because we "solve" the parameters of known functions. In this case the file name. No nasty opaque memory address.

If you are curious about all the slashes at the beginning, it is how Windows supports long path names; and Rust uses it by default.

After this you will get

```
0x75D03140 jmp dword [75D61024h] 
```

Which is a pretty print of the instruction that will be run. In this case there is nothing fancy to see.

But if we step just one instruction, the next one is more interesting. We see:

```
0x752B0E00 mov edi,edi - EDI=0- EDI=0
```

Not only the instruction, but also the register values. Much easier.

## Js Api

### init

```js
function init(path) { ... }
```

Starts the application at ```path```.

### go

```js
function go() { ... }
```

Returns running the application until it hits a breakpoint.

### step

```js
function step() { ... }
```

Runs just one assembly instruction

### addBreakpoint

```js
function addBreakpoint(location, once) { ... }
```

Adds a breakpoint at ```location```, that can be a memory address, or a function name.  
When using function name, it must be a function whose symbol is loaded.  

```once``` automatically deletes the breakpoint after its first hit.

### print

```js
function print(...arguments) { ... }
```

Pretty print anything to the specified output.

### currentStackFrame

```js
function currentStackFrame() { ... }
```

Returns all the details of the current stack frame.

### getThreadContext

```js
function getThreadContext() { ... }
```

Returns all the details of the current thread context. That includes registers.

### getCurrentInstructionString

```js
function getCurrentInstructionString() { ... }
```

Returns a friendly string with the current string. It contains the value of the registers and memory involved in the instruction.
