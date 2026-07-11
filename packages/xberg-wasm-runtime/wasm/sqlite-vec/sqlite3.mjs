/* @preserve
**
** LICENSE for the sqlite3 WebAssembly/JavaScript APIs.
**
** This bundle (typically released as sqlite3.js or sqlite3.mjs)
** is an amalgamation of JavaScript source code from two projects:
**
** 1) https://emscripten.org: the Emscripten "glue code" is covered by
**    the terms of the MIT license and University of Illinois/NCSA
**    Open Source License, as described at:
**
**    https://emscripten.org/docs/introducing_emscripten/emscripten_license.html
**
** 2) https://sqlite.org: all code and documentation labeled as being
**    from this source are released under the same terms as the sqlite3
**    C library:
**
** 2022-10-16
**
** The author disclaims copyright to this source code.  In place of a
** legal notice, here is a blessing:
**
** *   May you do good and not evil.
** *   May you find forgiveness for yourself and forgive others.
** *   May you share freely, never taking more than you give.
*/
/* @preserve
** This code was built from sqlite3 version...
**
** SQLITE_VERSION "3.53.3"
** SQLITE_VERSION_NUMBER 3053003
** SQLITE_SOURCE_ID "2026-06-26 20:14:12 d4c0e51e4aeb96955b99185ab9cde75c339e2c29c3f3f12428d364a10d782c62"
**
** Emscripten SDK: 3.1.74
*/

var sqlite3InitModule = (() => {
  var _scriptName = import.meta.url;
  
  return (
function(moduleArg = {}) {
  var moduleRtn;

// include: shell.js
// The Module object: Our interface to the outside world. We import
// and export values on it. There are various ways Module can be used:
// 1. Not defined. We create it here
// 2. A function parameter, function(moduleArg) => Promise<Module>
// 3. pre-run appended it, var Module = {}; ..generated code..
// 4. External script tag defines var Module.
// We need to check if Module already exists (e.g. case 3 above).
// Substitution will be replaced with actual code on later stage of the build,
// this way Closure Compiler will not mangle it (e.g. case 4. above).
// Note that if you want to run closure, and also to use Module
// after the generated code, you will need to define   var Module = {};
// before the code. Then that object will be used in the code, and you
// can continue to use Module afterwards as well.
var Module = moduleArg;

// Set up the promise that indicates the Module is initialized
var readyPromiseResolve, readyPromiseReject;
var readyPromise = new Promise((resolve, reject) => {
  readyPromiseResolve = resolve;
  readyPromiseReject = reject;
});
["_malloc","_free","_realloc","_sqlite3_aggregate_context","_sqlite3_auto_extension","_sqlite3_bind_blob","_sqlite3_bind_double","_sqlite3_bind_int","_sqlite3_bind_int64","_sqlite3_bind_null","_sqlite3_bind_parameter_count","_sqlite3_bind_parameter_index","_sqlite3_bind_parameter_name","_sqlite3_bind_pointer","_sqlite3_bind_text","_sqlite3_bind_zeroblob","_sqlite3_busy_handler","_sqlite3_busy_timeout","_sqlite3_cancel_auto_extension","_sqlite3_changes","_sqlite3_changes64","_sqlite3_clear_bindings","_sqlite3_close_v2","_sqlite3_collation_needed","_sqlite3_column_blob","_sqlite3_column_bytes","_sqlite3_column_count","_sqlite3_column_decltype","_sqlite3_column_double","_sqlite3_column_int","_sqlite3_column_int64","_sqlite3_column_name","_sqlite3_column_text","_sqlite3_column_type","_sqlite3_column_value","_sqlite3_commit_hook","_sqlite3_compileoption_get","_sqlite3_compileoption_used","_sqlite3_complete","_sqlite3_context_db_handle","_sqlite3_create_collation","_sqlite3_create_collation_v2","_sqlite3_create_function","_sqlite3_create_function_v2","_sqlite3_data_count","_sqlite3_db_filename","_sqlite3_db_handle","_sqlite3_db_name","_sqlite3_db_readonly","_sqlite3_db_status","_sqlite3_db_status64","_sqlite3_deserialize","_sqlite3_errcode","_sqlite3_errmsg","_sqlite3_error_offset","_sqlite3_errstr","_sqlite3_exec","_sqlite3_expanded_sql","_sqlite3_extended_errcode","_sqlite3_extended_result_codes","_sqlite3_file_control","_sqlite3_finalize","_sqlite3_free","_sqlite3_get_auxdata","_sqlite3_get_autocommit","_sqlite3_initialize","_sqlite3_interrupt","_sqlite3_is_interrupted","_sqlite3_keyword_count","_sqlite3_keyword_name","_sqlite3_keyword_check","_sqlite3_last_insert_rowid","_sqlite3_libversion","_sqlite3_libversion_number","_sqlite3_limit","_sqlite3_malloc","_sqlite3_malloc64","_sqlite3_msize","_sqlite3_next_stmt","_sqlite3_open","_sqlite3_open_v2","_sqlite3_overload_function","_sqlite3_prepare_v2","_sqlite3_prepare_v3","_sqlite3_randomness","_sqlite3_realloc","_sqlite3_realloc64","_sqlite3_reset","_sqlite3_reset_auto_extension","_sqlite3_result_blob","_sqlite3_result_double","_sqlite3_result_error","_sqlite3_result_error_code","_sqlite3_result_error_nomem","_sqlite3_result_error_toobig","_sqlite3_result_int","_sqlite3_result_int64","_sqlite3_result_null","_sqlite3_result_pointer","_sqlite3_result_subtype","_sqlite3_result_text","_sqlite3_result_zeroblob","_sqlite3_result_zeroblob64","_sqlite3_rollback_hook","_sqlite3_serialize","_sqlite3_set_auxdata","_sqlite3_set_last_insert_rowid","_sqlite3_set_errmsg","_sqlite3_shutdown","_sqlite3_sourceid","_sqlite3_sql","_sqlite3_status","_sqlite3_status64","_sqlite3_step","_sqlite3_stmt_busy","_sqlite3_stmt_explain","_sqlite3_stmt_isexplain","_sqlite3_stmt_readonly","_sqlite3_stmt_status","_sqlite3_strglob","_sqlite3_stricmp","_sqlite3_strlike","_sqlite3_strnicmp","_sqlite3_table_column_metadata","_sqlite3_total_changes","_sqlite3_total_changes64","_sqlite3_trace_v2","_sqlite3_txn_state","_sqlite3_update_hook","_sqlite3_uri_boolean","_sqlite3_uri_int64","_sqlite3_uri_key","_sqlite3_uri_parameter","_sqlite3_user_data","_sqlite3_value_blob","_sqlite3_value_bytes","_sqlite3_value_double","_sqlite3_value_dup","_sqlite3_value_free","_sqlite3_value_frombind","_sqlite3_value_int","_sqlite3_value_int64","_sqlite3_value_nochange","_sqlite3_value_numeric_type","_sqlite3_value_pointer","_sqlite3_value_subtype","_sqlite3_value_text","_sqlite3_value_type","_sqlite3_vfs_find","_sqlite3_vfs_register","_sqlite3_vfs_unregister","_sqlite3_vtab_collation","_sqlite3_vtab_distinct","_sqlite3_vtab_in","_sqlite3_vtab_in_first","_sqlite3_vtab_in_next","_sqlite3_vtab_nochange","_sqlite3_vtab_on_conflict","_sqlite3_vtab_rhs_value","_sqlite3_column_database_name","_sqlite3_column_origin_name","_sqlite3_column_table_name","_sqlite3_create_module","_sqlite3_create_module_v2","_sqlite3_create_window_function","_sqlite3_declare_vtab","_sqlite3_drop_modules","_sqlite3_preupdate_blobwrite","_sqlite3_preupdate_count","_sqlite3_preupdate_depth","_sqlite3_preupdate_hook","_sqlite3_preupdate_new","_sqlite3_preupdate_old","_sqlite3_progress_handler","_sqlite3_set_authorizer","_sqlite3changegroup_add","_sqlite3changegroup_add_strm","_sqlite3changegroup_delete","_sqlite3changegroup_new","_sqlite3changegroup_output","_sqlite3changegroup_output_strm","_sqlite3changeset_apply","_sqlite3changeset_apply_strm","_sqlite3changeset_apply_v2","_sqlite3changeset_apply_v2_strm","_sqlite3changeset_apply_v3","_sqlite3changeset_apply_v3_strm","_sqlite3changeset_concat","_sqlite3changeset_concat_strm","_sqlite3changeset_conflict","_sqlite3changeset_finalize","_sqlite3changeset_fk_conflicts","_sqlite3changeset_invert","_sqlite3changeset_invert_strm","_sqlite3changeset_new","_sqlite3changeset_next","_sqlite3changeset_old","_sqlite3changeset_op","_sqlite3changeset_pk","_sqlite3changeset_start","_sqlite3changeset_start_strm","_sqlite3changeset_start_v2","_sqlite3changeset_start_v2_strm","_sqlite3session_attach","_sqlite3session_changeset","_sqlite3session_changeset_size","_sqlite3session_changeset_strm","_sqlite3session_config","_sqlite3session_create","_sqlite3session_delete","_sqlite3session_diff","_sqlite3session_enable","_sqlite3session_indirect","_sqlite3session_isempty","_sqlite3session_memory_used","_sqlite3session_object_config","_sqlite3session_patchset","_sqlite3session_patchset_strm","_sqlite3session_table_filter","___indirect_function_table","_sqlite3__wasm_pstack_ptr","_sqlite3__wasm_pstack_restore","_sqlite3__wasm_pstack_alloc","_sqlite3__wasm_pstack_remaining","_sqlite3__wasm_pstack_quota","_sqlite3__wasm_test_struct","_sqlite3__wasm_enum_json","_sqlite3__wasm_vfs_unlink","_sqlite3__wasm_db_vfs","_sqlite3__wasm_db_reset","_sqlite3__wasm_db_export_chunked","_sqlite3__wasm_db_serialize","_sqlite3__wasm_vfs_create_file","_sqlite3__wasm_posix_create_file","_sqlite3__wasm_kvvfsMakeKey","_sqlite3__wasm_kvvfs_methods","_sqlite3__wasm_vtab_config","_sqlite3__wasm_db_config_ip","_sqlite3__wasm_db_config_pii","_sqlite3__wasm_db_config_s","_sqlite3__wasm_config_i","_sqlite3__wasm_config_ii","_sqlite3__wasm_config_j","_sqlite3__wasm_qfmt_token","_sqlite3__wasm_kvvfs_decode","_sqlite3__wasm_kvvfs_encode","_sqlite3__wasm_init_wasmfs","_sqlite3__wasm_test_intptr","_sqlite3__wasm_test_voidptr","_sqlite3__wasm_test_int64_max","_sqlite3__wasm_test_int64_min","_sqlite3__wasm_test_int64_times2","_sqlite3__wasm_test_int64_minmax","_sqlite3__wasm_test_int64ptr","_sqlite3__wasm_test_stack_overflow","_sqlite3__wasm_test_str_hello","_sqlite3__wasm_SQLTester_strglob","___set_stack_limits","onRuntimeInitialized"].forEach((prop) => {
  if (!Object.getOwnPropertyDescriptor(readyPromise, prop)) {
    Object.defineProperty(readyPromise, prop, {
      get: () => abort('You are getting ' + prop + ' on the Promise object, instead of the instance. Use .then() to get called back with the instance, see the MODULARIZE docs in src/settings.js'),
      set: () => abort('You are setting ' + prop + ' on the Promise object, instead of the instance. Use .then() to get called back with the instance, see the MODULARIZE docs in src/settings.js'),
    });
  }
});

// Determine the runtime environment we are in. You can customize this by
// setting the ENVIRONMENT setting at compile time (see settings.js).

// Attempt to auto-detect the environment
var ENVIRONMENT_IS_WEB = typeof window == 'object';
var ENVIRONMENT_IS_WORKER = typeof WorkerGlobalScope != 'undefined';
// N.b. Electron.js environment is simultaneously a NODE-environment, but
// also a web environment.
var ENVIRONMENT_IS_NODE = typeof process == 'object' && typeof process.versions == 'object' && typeof process.versions.node == 'string' && process.type != 'renderer';
var ENVIRONMENT_IS_SHELL = !ENVIRONMENT_IS_WEB && !ENVIRONMENT_IS_NODE && !ENVIRONMENT_IS_WORKER;

// --pre-jses are emitted after the Module integration code, so that they can
// refer to Module (if they choose; they can also define Module)
// include: /src/dist/.build/sqlite-src-3530300/ext/wasm/bld/pre-js.esm.js
/**
   BEGIN FILE: api/pre-js.js

   This file is intended to be prepended to the sqlite3.js build using
   Emscripten's --pre-js=THIS_FILE flag (or equivalent). It is run
   from inside of sqlite3InitModule(), after Emscripten's Module is
   defined, but early enough that we can ammend, or even outright
   replace, Module from here.

   Because this runs in-between Emscripten's own bootstrapping and
   Emscripten's main work, we must be careful with file-local symbol
   names. e.g. don't overwrite anything Emscripten defines and do not
   use 'const' for local symbols which Emscripten might try to use for
   itself. i.e. try to keep file-local symbol names obnoxiously
   collision-resistant.
*/
/**
   This file was preprocessed using:

   ./c-pp -o ./bld/pre-js.esm.js -Dtarget:es6-module -DModule.instantiateWasm api/pre-js.c-pp.js
*/
(function(Module){
  const sIMS =
        globalThis.sqlite3InitModuleState/*from extern-post-js.c-pp.js*/
        || Object.assign(Object.create(null),{
          /* In WASMFS builds this file gets loaded once per thread,
             but sqlite3InitModuleState is not getting set for the
             worker threads? That those workers seem to function fine
             despite that is curious. */
          debugModule: function(){
            console.warn("globalThis.sqlite3InitModuleState is missing",arguments);
          }
        });
  delete globalThis.sqlite3InitModuleState;
  sIMS.debugModule('pre-js.js sqlite3InitModuleState =',sIMS);

  /**
     This custom locateFile() tries to figure out where to load `path`
     from. The intent is to provide a way for foo/bar/X.js loaded from a
     Worker constructor or importScripts() to be able to resolve
     foo/bar/X.wasm (in the latter case, with some help):

     1) If URL param named the same as `path` is set, it is returned.

     2) If sqlite3InitModuleState.sqlite3Dir is set, then (thatName + path)
     is returned (it's assumed to end with '/').

     3) If this code is running in the main UI thread AND it was loaded
     from a SCRIPT tag, the directory part of that URL is used
     as the prefix. (This form of resolution unfortunately does not
     function for scripts loaded via importScripts().)

     4) If none of the above apply, (prefix+path) is returned.

     None of the above apply in ES6 builds, which uses a much simpler
     approach.
  */
  Module['locateFile'] = function(path, prefix) {
    if( this.emscriptenLocateFile instanceof Function ){
      /* [tag:locateFile] Client-overridden impl. We do not support
         this but offer it as a back-door which will go away the
         moment either Emscripten changes that interface or we manage
         to get non-Emscripten builds working.
         https://sqlite.org/forum/forumpost/1eec339854c935bd */
      return this.emscriptenLocateFile(path, prefix);
    }
    return new URL(path, import.meta.url).href;
  }.bind(sIMS);

  /**
     Override Module.instantiateWasm().

     A custom Module.instantiateWasm() does not work in WASMFS builds:

     https://github.com/emscripten-core/emscripten/issues/17951

     In such builds we must disable this.

     It's disabled in the (unsupported/untested) node builds because
     node does not do fetch().
  */
  Module['instantiateWasm'] = function callee(imports,onSuccess){
    if( this.emscriptenInstantiateWasm instanceof Function ){
      /* See [tag:locateFile]. Same story here */
      return this.emscriptenInstantiateWasm(imports, onSuccess);
    }
    const sims = this;
    const uri = Module.locateFile(
      sims.wasmFilename, (
        ('undefined'===typeof scriptDirectory/*var defined by Emscripten glue*/)
          ? "" : scriptDirectory)
    );
    sims.debugModule("instantiateWasm() uri =", uri, "sIMS =",this);
    const wfetch = ()=>fetch(uri, {credentials: 'same-origin'});
    const finalThen = (arg)=>{
      arg.imports = imports;
      sims.instantiateWasm = arg /* used by sqlite3-api-prologue.c-pp.js */;
      onSuccess(arg.instance, arg.module);
    };
    const loadWasm = WebAssembly.instantiateStreaming
          ? async ()=>
          WebAssembly
          .instantiateStreaming(wfetch(), imports)
          .then(finalThen)
          : async ()=>// Safari < v15
          wfetch()
          .then(response => response.arrayBuffer())
          .then(bytes => WebAssembly.instantiate(bytes, imports))
          .then(finalThen)
    return loadWasm();
  }.bind(sIMS);
})(Module);
/* END FILE: api/pre-js.js. */
// end include: /src/dist/.build/sqlite-src-3530300/ext/wasm/bld/pre-js.esm.js


// Sometimes an existing Module object exists with properties
// meant to overwrite the default module functionality. Here
// we collect those properties and reapply _after_ we configure
// the current environment's defaults to avoid having to be so
// defensive during initialization.
var moduleOverrides = Object.assign({}, Module);

var arguments_ = [];
var thisProgram = './this.program';
var quit_ = (status, toThrow) => {
  throw toThrow;
};

// `/` should be present at the end if `scriptDirectory` is not empty
var scriptDirectory = '';
function locateFile(path) {
  if (Module['locateFile']) {
    return Module['locateFile'](path, scriptDirectory);
  }
  return scriptDirectory + path;
}

// Hooks that are implemented differently in different runtime environments.
var readAsync, readBinary;

if (ENVIRONMENT_IS_SHELL) {

  if ((typeof process == 'object' && typeof require === 'function') || typeof window == 'object' || typeof WorkerGlobalScope != 'undefined') throw new Error('not compiled for this environment (did you build to HTML and try to run it not on the web, or set ENVIRONMENT to something - like node - and run it someplace else - like on the web?)');

} else

// Note that this includes Node.js workers when relevant (pthreads is enabled).
// Node.js workers are detected as a combination of ENVIRONMENT_IS_WORKER and
// ENVIRONMENT_IS_NODE.
if (ENVIRONMENT_IS_WEB || ENVIRONMENT_IS_WORKER) {
  if (ENVIRONMENT_IS_WORKER) { // Check worker, not web, since window could be polyfilled
    scriptDirectory = self.location.href;
  } else if (typeof document != 'undefined' && document.currentScript) { // web
    scriptDirectory = document.currentScript.src;
  }
  // When MODULARIZE, this JS may be executed later, after document.currentScript
  // is gone, so we saved it, and we use it here instead of any other info.
  if (_scriptName) {
    scriptDirectory = _scriptName;
  }
  // blob urls look like blob:http://site.com/etc/etc and we cannot infer anything from them.
  // otherwise, slice off the final part of the url to find the script directory.
  // if scriptDirectory does not contain a slash, lastIndexOf will return -1,
  // and scriptDirectory will correctly be replaced with an empty string.
  // If scriptDirectory contains a query (starting with ?) or a fragment (starting with #),
  // they are removed because they could contain a slash.
  if (scriptDirectory.startsWith('blob:')) {
    scriptDirectory = '';
  } else {
    scriptDirectory = scriptDirectory.substr(0, scriptDirectory.replace(/[?#].*/, '').lastIndexOf('/')+1);
  }

  if (!(typeof window == 'object' || typeof WorkerGlobalScope != 'undefined')) throw new Error('not compiled for this environment (did you build to HTML and try to run it not on the web, or set ENVIRONMENT to something - like node - and run it someplace else - like on the web?)');

  {
// include: web_or_worker_shell_read.js
if (ENVIRONMENT_IS_WORKER) {
    readBinary = (url) => {
      var xhr = new XMLHttpRequest();
      xhr.open('GET', url, false);
      xhr.responseType = 'arraybuffer';
      xhr.send(null);
      return new Uint8Array(/** @type{!ArrayBuffer} */(xhr.response));
    };
  }

  readAsync = async (url) => {
    assert(!isFileURI(url), "readAsync does not work with file:// URLs");
    var response = await fetch(url, { credentials: 'same-origin' });
    if (response.ok) {
      return response.arrayBuffer();
    }
    throw new Error(response.status + ' : ' + response.url);
  };
// end include: web_or_worker_shell_read.js
  }
} else
{
  throw new Error('environment detection error');
}

var out = Module['print'] || console.log.bind(console);
var err = Module['printErr'] || console.error.bind(console);

// Merge back in the overrides
Object.assign(Module, moduleOverrides);
// Free the object hierarchy contained in the overrides, this lets the GC
// reclaim data used.
moduleOverrides = null;
checkIncomingModuleAPI();

// Emit code to handle expected values on the Module object. This applies Module.x
// to the proper local x. This has two benefits: first, we only emit it if it is
// expected to arrive, and second, by using a local everywhere else that can be
// minified.

if (Module['arguments']) arguments_ = Module['arguments'];legacyModuleProp('arguments', 'arguments_');

if (Module['thisProgram']) thisProgram = Module['thisProgram'];legacyModuleProp('thisProgram', 'thisProgram');

// perform assertions in shell.js after we set up out() and err(), as otherwise if an assertion fails it cannot print the message
// Assertions on removed incoming Module JS APIs.
assert(typeof Module['memoryInitializerPrefixURL'] == 'undefined', 'Module.memoryInitializerPrefixURL option was removed, use Module.locateFile instead');
assert(typeof Module['pthreadMainPrefixURL'] == 'undefined', 'Module.pthreadMainPrefixURL option was removed, use Module.locateFile instead');
assert(typeof Module['cdInitializerPrefixURL'] == 'undefined', 'Module.cdInitializerPrefixURL option was removed, use Module.locateFile instead');
assert(typeof Module['filePackagePrefixURL'] == 'undefined', 'Module.filePackagePrefixURL option was removed, use Module.locateFile instead');
assert(typeof Module['read'] == 'undefined', 'Module.read option was removed');
assert(typeof Module['readAsync'] == 'undefined', 'Module.readAsync option was removed (modify readAsync in JS)');
assert(typeof Module['readBinary'] == 'undefined', 'Module.readBinary option was removed (modify readBinary in JS)');
assert(typeof Module['setWindowTitle'] == 'undefined', 'Module.setWindowTitle option was removed (modify emscripten_set_window_title in JS)');
assert(typeof Module['TOTAL_MEMORY'] == 'undefined', 'Module.TOTAL_MEMORY has been renamed Module.INITIAL_MEMORY');
legacyModuleProp('asm', 'wasmExports');
legacyModuleProp('readAsync', 'readAsync');
legacyModuleProp('readBinary', 'readBinary');
legacyModuleProp('setWindowTitle', 'setWindowTitle');
var IDBFS = 'IDBFS is no longer included by default; build with -lidbfs.js';
var PROXYFS = 'PROXYFS is no longer included by default; build with -lproxyfs.js';
var WORKERFS = 'WORKERFS is no longer included by default; build with -lworkerfs.js';
var FETCHFS = 'FETCHFS is no longer included by default; build with -lfetchfs.js';
var ICASEFS = 'ICASEFS is no longer included by default; build with -licasefs.js';
var JSFILEFS = 'JSFILEFS is no longer included by default; build with -ljsfilefs.js';
var OPFS = 'OPFS is no longer included by default; build with -lopfs.js';

var NODEFS = 'NODEFS is no longer included by default; build with -lnodefs.js';

assert(!ENVIRONMENT_IS_NODE, 'node environment detected but not enabled at build time.  Add `node` to `-sENVIRONMENT` to enable.');

assert(!ENVIRONMENT_IS_SHELL, 'shell environment detected but not enabled at build time.  Add `shell` to `-sENVIRONMENT` to enable.');

// end include: shell.js

// include: preamble.js
// === Preamble library stuff ===

// Documentation for the public APIs defined in this file must be updated in:
//    site/source/docs/api_reference/preamble.js.rst
// A prebuilt local version of the documentation is available at:
//    site/build/text/docs/api_reference/preamble.js.txt
// You can also build docs locally as HTML or other formats in site/
// An online HTML version (which may be of a different version of Emscripten)
//    is up at http://kripken.github.io/emscripten-site/docs/api_reference/preamble.js.html

var wasmBinary = Module['wasmBinary'];legacyModuleProp('wasmBinary', 'wasmBinary');

if (typeof WebAssembly != 'object') {
  err('no native wasm support detected');
}

// Wasm globals

var wasmMemory;

//========================================
// Runtime essentials
//========================================

// whether we are quitting the application. no code should run after this.
// set in exit() and abort()
var ABORT = false;

// set by exit() and abort().  Passed to 'onExit' handler.
// NOTE: This is also used as the process return code code in shell environments
// but only when noExitRuntime is false.
var EXITSTATUS;

// In STRICT mode, we only define assert() when ASSERTIONS is set.  i.e. we
// don't define it at all in release modes.  This matches the behaviour of
// MINIMAL_RUNTIME.
// TODO(sbc): Make this the default even without STRICT enabled.
/** @type {function(*, string=)} */
function assert(condition, text) {
  if (!condition) {
    abort('Assertion failed' + (text ? ': ' + text : ''));
  }
}

// We used to include malloc/free by default in the past. Show a helpful error in
// builds with assertions.

// Memory management

var HEAP,
/** @type {!Int8Array} */
  HEAP8,
/** @type {!Uint8Array} */
  HEAPU8,
/** @type {!Int16Array} */
  HEAP16,
/** @type {!Uint16Array} */
  HEAPU16,
/** @type {!Int32Array} */
  HEAP32,
/** @type {!Uint32Array} */
  HEAPU32,
/** @type {!Float32Array} */
  HEAPF32,
/* BigInt64Array type is not correctly defined in closure
/** not-@type {!BigInt64Array} */
  HEAP64,
/* BigUint64Array type is not correctly defined in closure
/** not-t@type {!BigUint64Array} */
  HEAPU64,
/** @type {!Float64Array} */
  HEAPF64;

// include: runtime_shared.js
function updateMemoryViews() {
  var b = wasmMemory.buffer;
  Module['HEAP8'] = HEAP8 = new Int8Array(b);
  Module['HEAP16'] = HEAP16 = new Int16Array(b);
  Module['HEAPU8'] = HEAPU8 = new Uint8Array(b);
  Module['HEAPU16'] = HEAPU16 = new Uint16Array(b);
  Module['HEAP32'] = HEAP32 = new Int32Array(b);
  Module['HEAPU32'] = HEAPU32 = new Uint32Array(b);
  Module['HEAPF32'] = HEAPF32 = new Float32Array(b);
  Module['HEAPF64'] = HEAPF64 = new Float64Array(b);
  Module['HEAP64'] = HEAP64 = new BigInt64Array(b);
  Module['HEAPU64'] = HEAPU64 = new BigUint64Array(b);
}

// end include: runtime_shared.js
assert(!Module['STACK_SIZE'], 'STACK_SIZE can no longer be set at runtime.  Use -sSTACK_SIZE at link time')

assert(typeof Int32Array != 'undefined' && typeof Float64Array !== 'undefined' && Int32Array.prototype.subarray != undefined && Int32Array.prototype.set != undefined,
       'JS engine does not provide full typed array support');

// In non-standalone/normal mode, we create the memory here.
// include: runtime_init_memory.js
// Create the wasm memory. (Note: this only applies if IMPORTED_MEMORY is defined)

// check for full engine support (use string 'subarray' to avoid closure compiler confusion)

  if (Module['wasmMemory']) {
    wasmMemory = Module['wasmMemory'];
  } else
  {
    var INITIAL_MEMORY = Module['INITIAL_MEMORY'] || 8388608;legacyModuleProp('INITIAL_MEMORY', 'INITIAL_MEMORY');

    assert(INITIAL_MEMORY >= 524288, 'INITIAL_MEMORY should be larger than STACK_SIZE, was ' + INITIAL_MEMORY + '! (STACK_SIZE=' + 524288 + ')');
    /** @suppress {checkTypes} */
    wasmMemory = new WebAssembly.Memory({
      'initial': INITIAL_MEMORY / 65536,
      // In theory we should not need to emit the maximum if we want "unlimited"
      // or 4GB of memory, but VMs error on that atm, see
      // https://github.com/emscripten-core/emscripten/issues/14130
      // And in the pthreads case we definitely need to emit a maximum. So
      // always emit one.
      'maximum': 32768,
    });
  }

  updateMemoryViews();

// end include: runtime_init_memory.js

// include: runtime_stack_check.js
// Initializes the stack cookie. Called at the startup of main and at the startup of each thread in pthreads mode.
function writeStackCookie() {
  var max = _emscripten_stack_get_end();
  assert((max & 3) == 0);
  // If the stack ends at address zero we write our cookies 4 bytes into the
  // stack.  This prevents interference with SAFE_HEAP and ASAN which also
  // monitor writes to address zero.
  if (max == 0) {
    max += 4;
  }
  // The stack grow downwards towards _emscripten_stack_get_end.
  // We write cookies to the final two words in the stack and detect if they are
  // ever overwritten.
  HEAPU32[((max)>>2)] = 0x02135467;checkInt32(0x02135467);
  HEAPU32[(((max)+(4))>>2)] = 0x89BACDFE;checkInt32(0x89BACDFE);
  // Also test the global address 0 for integrity.
  HEAPU32[((0)>>2)] = 1668509029;checkInt32(1668509029);
}

function checkStackCookie() {
  if (ABORT) return;
  var max = _emscripten_stack_get_end();
  // See writeStackCookie().
  if (max == 0) {
    max += 4;
  }
  var cookie1 = HEAPU32[((max)>>2)];
  var cookie2 = HEAPU32[(((max)+(4))>>2)];
  if (cookie1 != 0x02135467 || cookie2 != 0x89BACDFE) {
    abort(`Stack overflow! Stack cookie has been overwritten at ${ptrToString(max)}, expected hex dwords 0x89BACDFE and 0x2135467, but received ${ptrToString(cookie2)} ${ptrToString(cookie1)}`);
  }
  // Also test the global address 0 for integrity.
  if (HEAPU32[((0)>>2)] != 0x63736d65 /* 'emsc' */) {
    abort('Runtime error: The application has corrupted its heap memory area (address zero)!');
  }
}
// end include: runtime_stack_check.js
var __ATPRERUN__  = []; // functions called before the runtime is initialized
var __ATINIT__    = []; // functions called during startup
var __ATEXIT__    = []; // functions called during shutdown
var __ATPOSTRUN__ = []; // functions called after the main() is called

var runtimeInitialized = false;

function preRun() {
  if (Module['preRun']) {
    if (typeof Module['preRun'] == 'function') Module['preRun'] = [Module['preRun']];
    while (Module['preRun'].length) {
      addOnPreRun(Module['preRun'].shift());
    }
  }
  callRuntimeCallbacks(__ATPRERUN__);
}

function initRuntime() {
  assert(!runtimeInitialized);
  runtimeInitialized = true;

  checkStackCookie();

  setStackLimits();

  
if (!Module['noFSInit'] && !FS.initialized)
  FS.init();
FS.ignorePermissions = false;

TTY.init();
  callRuntimeCallbacks(__ATINIT__);
}

function postRun() {
  checkStackCookie();

  if (Module['postRun']) {
    if (typeof Module['postRun'] == 'function') Module['postRun'] = [Module['postRun']];
    while (Module['postRun'].length) {
      addOnPostRun(Module['postRun'].shift());
    }
  }

  callRuntimeCallbacks(__ATPOSTRUN__);
}

function addOnPreRun(cb) {
  __ATPRERUN__.unshift(cb);
}

function addOnInit(cb) {
  __ATINIT__.unshift(cb);
}

function addOnExit(cb) {
}

function addOnPostRun(cb) {
  __ATPOSTRUN__.unshift(cb);
}

// include: runtime_math.js
// https://developer.mozilla.org/en-US/docs/Web/JavaScript/Reference/Global_Objects/Math/imul

// https://developer.mozilla.org/en-US/docs/Web/JavaScript/Reference/Global_Objects/Math/fround

// https://developer.mozilla.org/en-US/docs/Web/JavaScript/Reference/Global_Objects/Math/clz32

// https://developer.mozilla.org/en-US/docs/Web/JavaScript/Reference/Global_Objects/Math/trunc

assert(Math.imul, 'This browser does not support Math.imul(), build with LEGACY_VM_SUPPORT or POLYFILL_OLD_MATH_FUNCTIONS to add in a polyfill');
assert(Math.fround, 'This browser does not support Math.fround(), build with LEGACY_VM_SUPPORT or POLYFILL_OLD_MATH_FUNCTIONS to add in a polyfill');
assert(Math.clz32, 'This browser does not support Math.clz32(), build with LEGACY_VM_SUPPORT or POLYFILL_OLD_MATH_FUNCTIONS to add in a polyfill');
assert(Math.trunc, 'This browser does not support Math.trunc(), build with LEGACY_VM_SUPPORT or POLYFILL_OLD_MATH_FUNCTIONS to add in a polyfill');
// end include: runtime_math.js
// A counter of dependencies for calling run(). If we need to
// do asynchronous work before running, increment this and
// decrement it. Incrementing must happen in a place like
// Module.preRun (used by emcc to add file preloading).
// Note that you can add dependencies in preRun, even though
// it happens right before run - run will be postponed until
// the dependencies are met.
var runDependencies = 0;
var dependenciesFulfilled = null; // overridden to take different actions when all run dependencies are fulfilled
var runDependencyTracking = {};
var runDependencyWatcher = null;

function getUniqueRunDependency(id) {
  var orig = id;
  while (1) {
    if (!runDependencyTracking[id]) return id;
    id = orig + Math.random();
  }
}

function addRunDependency(id) {
  runDependencies++;

  Module['monitorRunDependencies']?.(runDependencies);

  if (id) {
    assert(!runDependencyTracking[id]);
    runDependencyTracking[id] = 1;
    if (runDependencyWatcher === null && typeof setInterval != 'undefined') {
      // Check for missing dependencies every few seconds
      runDependencyWatcher = setInterval(() => {
        if (ABORT) {
          clearInterval(runDependencyWatcher);
          runDependencyWatcher = null;
          return;
        }
        var shown = false;
        for (var dep in runDependencyTracking) {
          if (!shown) {
            shown = true;
            err('still waiting on run dependencies:');
          }
          err(`dependency: ${dep}`);
        }
        if (shown) {
          err('(end of list)');
        }
      }, 10000);
    }
  } else {
    err('warning: run dependency added without ID');
  }
}

function removeRunDependency(id) {
  runDependencies--;

  Module['monitorRunDependencies']?.(runDependencies);

  if (id) {
    assert(runDependencyTracking[id]);
    delete runDependencyTracking[id];
  } else {
    err('warning: run dependency removed without ID');
  }
  if (runDependencies == 0) {
    if (runDependencyWatcher !== null) {
      clearInterval(runDependencyWatcher);
      runDependencyWatcher = null;
    }
    if (dependenciesFulfilled) {
      var callback = dependenciesFulfilled;
      dependenciesFulfilled = null;
      callback(); // can add another dependenciesFulfilled
    }
  }
}

/** @param {string|number=} what */
function abort(what) {
  Module['onAbort']?.(what);

  what = 'Aborted(' + what + ')';
  // TODO(sbc): Should we remove printing and leave it up to whoever
  // catches the exception?
  err(what);

  ABORT = true;

  // Use a wasm runtime error, because a JS error might be seen as a foreign
  // exception, which means we'd run destructors on it. We need the error to
  // simply make the program stop.
  // FIXME This approach does not work in Wasm EH because it currently does not assume
  // all RuntimeErrors are from traps; it decides whether a RuntimeError is from
  // a trap or not based on a hidden field within the object. So at the moment
  // we don't have a way of throwing a wasm trap from JS. TODO Make a JS API that
  // allows this in the wasm spec.

  // Suppress closure compiler warning here. Closure compiler's builtin extern
  // definition for WebAssembly.RuntimeError claims it takes no arguments even
  // though it can.
  // TODO(https://github.com/google/closure-compiler/pull/3913): Remove if/when upstream closure gets fixed.
  /** @suppress {checkTypes} */
  var e = new WebAssembly.RuntimeError(what);

  readyPromiseReject(e);
  // Throw the error whether or not MODULARIZE is set because abort is used
  // in code paths apart from instantiation where an exception is expected
  // to be thrown when abort is called.
  throw e;
}

// include: memoryprofiler.js
// end include: memoryprofiler.js
// include: URIUtils.js
// Prefix of data URIs emitted by SINGLE_FILE and related options.
var dataURIPrefix = 'data:application/octet-stream;base64,';

/**
 * Indicates whether filename is a base64 data URI.
 * @noinline
 */
var isDataURI = (filename) => filename.startsWith(dataURIPrefix);

/**
 * Indicates whether filename is delivered via file protocol (as opposed to http/https)
 * @noinline
 */
var isFileURI = (filename) => filename.startsWith('file://');
// end include: URIUtils.js
function createExportWrapper(name, nargs) {
  return (...args) => {
    assert(runtimeInitialized, `native function \`${name}\` called before runtime initialization`);
    var f = wasmExports[name];
    assert(f, `exported native function \`${name}\` not found`);
    // Only assert for too many arguments. Too few can be valid since the missing arguments will be zero filled.
    assert(args.length <= nargs, `native function \`${name}\` called with ${args.length} args but expects ${nargs}`);
    return f(...args);
  };
}

// include: runtime_exceptions.js
// end include: runtime_exceptions.js
function findWasmBinary() {
  if (Module['locateFile']) {
    var f = 'sqlite3.wasm';
    if (!isDataURI(f)) {
      return locateFile(f);
    }
    return f;
  }
  // Use bundler-friendly `new URL(..., import.meta.url)` pattern; works in browsers too.
  return new URL('sqlite3.wasm', import.meta.url).href;
}

var wasmBinaryFile;

function getBinarySync(file) {
  if (file == wasmBinaryFile && wasmBinary) {
    return new Uint8Array(wasmBinary);
  }
  if (readBinary) {
    return readBinary(file);
  }
  throw 'both async and sync fetching of the wasm failed';
}

async function getWasmBinary(binaryFile) {
  // If we don't have the binary yet, load it asynchronously using readAsync.
  if (!wasmBinary
      ) {
    // Fetch the binary using readAsync
    try {
      var response = await readAsync(binaryFile);
      return new Uint8Array(response);
    } catch {
      // Fall back to getBinarySync below;
    }
  }

  // Otherwise, getBinarySync should be able to get it synchronously
  return getBinarySync(binaryFile);
}

async function instantiateArrayBuffer(binaryFile, imports) {
  try {
    var binary = await getWasmBinary(binaryFile);
    var instance = await WebAssembly.instantiate(binary, imports);
    return instance;
  } catch (reason) {
    err(`failed to asynchronously prepare wasm: ${reason}`);

    // Warn on some common problems.
    if (isFileURI(wasmBinaryFile)) {
      err(`warning: Loading from a file URI (${wasmBinaryFile}) is not supported in most browsers. See https://emscripten.org/docs/getting_started/FAQ.html#how-do-i-run-a-local-webserver-for-testing-why-does-my-program-stall-in-downloading-or-preparing`);
    }
    abort(reason);
  }
}

async function instantiateAsync(binary, binaryFile, imports) {
  if (!binary &&
      typeof WebAssembly.instantiateStreaming == 'function' &&
      !isDataURI(binaryFile) &&
      typeof fetch == 'function') {
    try {
      var response = fetch(binaryFile, { credentials: 'same-origin' });
      var instantiationResult = await WebAssembly.instantiateStreaming(response, imports);
      return instantiationResult;
    } catch (reason) {
      // We expect the most common failure cause to be a bad MIME type for the binary,
      // in which case falling back to ArrayBuffer instantiation should work.
      err(`wasm streaming compile failed: ${reason}`);
      err('falling back to ArrayBuffer instantiation');
      // fall back of instantiateArrayBuffer below
    };
  }
  return instantiateArrayBuffer(binaryFile, imports);
}

function getWasmImports() {
  // prepare imports
  return {
    'env': wasmImports,
    'wasi_snapshot_preview1': wasmImports,
  }
}

// Create the wasm instance.
// Receives the wasm imports, returns the exports.
async function createWasm() {
  // Load the wasm module and create an instance of using native support in the JS engine.
  // handle a generated wasm instance, receiving its exports and
  // performing other necessary setup
  /** @param {WebAssembly.Module=} module*/
  function receiveInstance(instance, module) {
    wasmExports = instance.exports;

    

    addOnInit(wasmExports['__wasm_call_ctors']);

    removeRunDependency('wasm-instantiate');
    return wasmExports;
  }
  // wait for the pthread pool (if any)
  addRunDependency('wasm-instantiate');

  // Prefer streaming instantiation if available.
  // Async compilation can be confusing when an error on the page overwrites Module
  // (for example, if the order of elements is wrong, and the one defining Module is
  // later), so we save Module and check it later.
  var trueModule = Module;
  function receiveInstantiationResult(result) {
    // 'result' is a ResultObject object which has both the module and instance.
    // receiveInstance() will swap in the exports (to Module.asm) so they can be called
    assert(Module === trueModule, 'the Module object should not be replaced during async compilation - perhaps the order of HTML elements is wrong?');
    trueModule = null;
    // TODO: Due to Closure regression https://github.com/google/closure-compiler/issues/3193, the above line no longer optimizes out down to the following line.
    // When the regression is fixed, can restore the above PTHREADS-enabled path.
    receiveInstance(result['instance']);
  }

  var info = getWasmImports();

  // User shell pages can write their own Module.instantiateWasm = function(imports, successCallback) callback
  // to manually instantiate the Wasm module themselves. This allows pages to
  // run the instantiation parallel to any other async startup actions they are
  // performing.
  // Also pthreads and wasm workers initialize the wasm instance through this
  // path.
  if (Module['instantiateWasm']) {
    try {
      return Module['instantiateWasm'](info, receiveInstance);
    } catch(e) {
      err(`Module.instantiateWasm callback failed with error: ${e}`);
        // If instantiation fails, reject the module ready promise.
        readyPromiseReject(e);
    }
  }

  wasmBinaryFile ??= findWasmBinary();

  try {
    var result = await instantiateAsync(wasmBinary, wasmBinaryFile, info);
    receiveInstantiationResult(result);
    return result;
  } catch (e) {
    // If instantiation fails, reject the module ready promise.
    readyPromiseReject(e);
    return;
  }
}

// include: runtime_debug.js
// Endianness check
(() => {
  var h16 = new Int16Array(1);
  var h8 = new Int8Array(h16.buffer);
  h16[0] = 0x6373;
  if (h8[0] !== 0x73 || h8[1] !== 0x63) throw 'Runtime error: expected the system to be little-endian! (Run with -sSUPPORT_BIG_ENDIAN to bypass)';
})();

if (Module['ENVIRONMENT']) {
  throw new Error('Module.ENVIRONMENT has been deprecated. To force the environment, use the ENVIRONMENT compile-time option (for example, -sENVIRONMENT=web or -sENVIRONMENT=node)');
}

function legacyModuleProp(prop, newName, incoming=true) {
  if (!Object.getOwnPropertyDescriptor(Module, prop)) {
    Object.defineProperty(Module, prop, {
      configurable: true,
      get() {
        let extra = incoming ? ' (the initial value can be provided on Module, but after startup the value is only looked for on a local variable of that name)' : '';
        abort(`\`Module.${prop}\` has been replaced by \`${newName}\`` + extra);

      }
    });
  }
}

function ignoredModuleProp(prop) {
  if (Object.getOwnPropertyDescriptor(Module, prop)) {
    abort(`\`Module.${prop}\` was supplied but \`${prop}\` not included in INCOMING_MODULE_JS_API`);
  }
}

// forcing the filesystem exports a few things by default
function isExportedByForceFilesystem(name) {
  return name === 'FS_createPath' ||
         name === 'FS_createDataFile' ||
         name === 'FS_createPreloadedFile' ||
         name === 'FS_unlink' ||
         name === 'addRunDependency' ||
         // The old FS has some functionality that WasmFS lacks.
         name === 'FS_createLazyFile' ||
         name === 'FS_createDevice' ||
         name === 'removeRunDependency';
}

/**
 * Intercept access to a global symbol.  This enables us to give informative
 * warnings/errors when folks attempt to use symbols they did not include in
 * their build, or no symbols that no longer exist.
 */
function hookGlobalSymbolAccess(sym, func) {
  if (typeof globalThis != 'undefined' && !Object.getOwnPropertyDescriptor(globalThis, sym)) {
    Object.defineProperty(globalThis, sym, {
      configurable: true,
      get() {
        func();
        return undefined;
      }
    });
  }
}

function missingGlobal(sym, msg) {
  hookGlobalSymbolAccess(sym, () => {
    warnOnce(`\`${sym}\` is not longer defined by emscripten. ${msg}`);
  });
}

missingGlobal('buffer', 'Please use HEAP8.buffer or wasmMemory.buffer');
missingGlobal('asm', 'Please use wasmExports instead');

function missingLibrarySymbol(sym) {
  hookGlobalSymbolAccess(sym, () => {
    // Can't `abort()` here because it would break code that does runtime
    // checks.  e.g. `if (typeof SDL === 'undefined')`.
    var msg = `\`${sym}\` is a library symbol and not included by default; add it to your library.js __deps or to DEFAULT_LIBRARY_FUNCS_TO_INCLUDE on the command line`;
    // DEFAULT_LIBRARY_FUNCS_TO_INCLUDE requires the name as it appears in
    // library.js, which means $name for a JS name with no prefix, or name
    // for a JS name like _name.
    var librarySymbol = sym;
    if (!librarySymbol.startsWith('_')) {
      librarySymbol = '$' + sym;
    }
    msg += ` (e.g. -sDEFAULT_LIBRARY_FUNCS_TO_INCLUDE='${librarySymbol}')`;
    if (isExportedByForceFilesystem(sym)) {
      msg += '. Alternatively, forcing filesystem support (-sFORCE_FILESYSTEM) can export this for you';
    }
    warnOnce(msg);
  });

  // Any symbol that is not included from the JS library is also (by definition)
  // not exported on the Module object.
  unexportedRuntimeSymbol(sym);
}

function unexportedRuntimeSymbol(sym) {
  if (!Object.getOwnPropertyDescriptor(Module, sym)) {
    Object.defineProperty(Module, sym, {
      configurable: true,
      get() {
        var msg = `'${sym}' was not exported. add it to EXPORTED_RUNTIME_METHODS (see the Emscripten FAQ)`;
        if (isExportedByForceFilesystem(sym)) {
          msg += '. Alternatively, forcing filesystem support (-sFORCE_FILESYSTEM) can export this for you';
        }
        abort(msg);
      }
    });
  }
}

var MAX_UINT8  = (2 **  8) - 1;
var MAX_UINT16 = (2 ** 16) - 1;
var MAX_UINT32 = (2 ** 32) - 1;
var MAX_UINT53 = (2 ** 53) - 1;
var MAX_UINT64 = (2 ** 64) - 1;

var MIN_INT8  = - (2 ** ( 8 - 1));
var MIN_INT16 = - (2 ** (16 - 1));
var MIN_INT32 = - (2 ** (32 - 1));
var MIN_INT53 = - (2 ** (53 - 1));
var MIN_INT64 = - (2 ** (64 - 1));

function checkInt(value, bits, min, max) {
  assert(Number.isInteger(Number(value)), `attempt to write non-integer (${value}) into integer heap`);
  assert(value <= max, `value (${value}) too large to write as ${bits}-bit value`);
  assert(value >= min, `value (${value}) too small to write as ${bits}-bit value`);
}

var checkInt1 = (value) => checkInt(value, 1, 1);
var checkInt8 = (value) => checkInt(value, 8, MIN_INT8, MAX_UINT8);
var checkInt16 = (value) => checkInt(value, 16, MIN_INT16, MAX_UINT16);
var checkInt32 = (value) => checkInt(value, 32, MIN_INT32, MAX_UINT32);
var checkInt53 = (value) => checkInt(value, 53, MIN_INT53, MAX_UINT53);
var checkInt64 = (value) => checkInt(value, 64, MIN_INT64, MAX_UINT64);

// Used by XXXXX_DEBUG settings to output debug messages.
function dbg(...args) {
  // TODO(sbc): Make this configurable somehow.  Its not always convenient for
  // logging to show up as warnings.
  console.warn(...args);
}
// end include: runtime_debug.js
// === Body ===
// end include: preamble.js


  class ExitStatus {
      name = 'ExitStatus';
      constructor(status) {
        this.message = `Program terminated with exit(${status})`;
        this.status = status;
      }
    }

  var callRuntimeCallbacks = (callbacks) => {
      while (callbacks.length > 0) {
        // Pass the module as the first argument.
        callbacks.shift()(Module);
      }
    };

  
    /**
     * @param {number} ptr
     * @param {string} type
     */
  function getValue(ptr, type = 'i8') {
    if (type.endsWith('*')) type = '*';
    switch (type) {
      case 'i1': return HEAP8[ptr];
      case 'i8': return HEAP8[ptr];
      case 'i16': return HEAP16[((ptr)>>1)];
      case 'i32': return HEAP32[((ptr)>>2)];
      case 'i64': return HEAP64[((ptr)>>3)];
      case 'float': return HEAPF32[((ptr)>>2)];
      case 'double': return HEAPF64[((ptr)>>3)];
      case '*': return HEAPU32[((ptr)>>2)];
      default: abort(`invalid type for getValue: ${type}`);
    }
  }

  var noExitRuntime = Module['noExitRuntime'] || true;

  var ptrToString = (ptr) => {
      assert(typeof ptr === 'number');
      // With CAN_ADDRESS_2GB or MEMORY64, pointers are already unsigned.
      ptr >>>= 0;
      return '0x' + ptr.toString(16).padStart(8, '0');
    };

  var setStackLimits = () => {
      var stackLow = _emscripten_stack_get_base();
      var stackHigh = _emscripten_stack_get_end();
      ___set_stack_limits(stackLow, stackHigh);
    };

  
    /**
     * @param {number} ptr
     * @param {number} value
     * @param {string} type
     */
  function setValue(ptr, value, type = 'i8') {
    if (type.endsWith('*')) type = '*';
    switch (type) {
      case 'i1': HEAP8[ptr] = value;checkInt8(value); break;
      case 'i8': HEAP8[ptr] = value;checkInt8(value); break;
      case 'i16': HEAP16[((ptr)>>1)] = value;checkInt16(value); break;
      case 'i32': HEAP32[((ptr)>>2)] = value;checkInt32(value); break;
      case 'i64': HEAP64[((ptr)>>3)] = BigInt(value);checkInt64(value); break;
      case 'float': HEAPF32[((ptr)>>2)] = value; break;
      case 'double': HEAPF64[((ptr)>>3)] = value; break;
      case '*': HEAPU32[((ptr)>>2)] = value; break;
      default: abort(`invalid type for setValue: ${type}`);
    }
  }

  var stackRestore = (val) => __emscripten_stack_restore(val);

  var stackSave = () => _emscripten_stack_get_current();

  var warnOnce = (text) => {
      warnOnce.shown ||= {};
      if (!warnOnce.shown[text]) {
        warnOnce.shown[text] = 1;
        err(text);
      }
    };

  var UTF8Decoder = typeof TextDecoder != 'undefined' ? new TextDecoder() : undefined;
  
    /**
     * Given a pointer 'idx' to a null-terminated UTF8-encoded string in the given
     * array that contains uint8 values, returns a copy of that string as a
     * Javascript String object.
     * heapOrArray is either a regular array, or a JavaScript typed array view.
     * @param {number=} idx
     * @param {number=} maxBytesToRead
     * @return {string}
     */
  var UTF8ArrayToString = (heapOrArray, idx = 0, maxBytesToRead = NaN) => {
      var endIdx = idx + maxBytesToRead;
      var endPtr = idx;
      // TextDecoder needs to know the byte length in advance, it doesn't stop on
      // null terminator by itself.  Also, use the length info to avoid running tiny
      // strings through TextDecoder, since .subarray() allocates garbage.
      // (As a tiny code save trick, compare endPtr against endIdx using a negation,
      // so that undefined/NaN means Infinity)
      while (heapOrArray[endPtr] && !(endPtr >= endIdx)) ++endPtr;
  
      if (endPtr - idx > 16 && heapOrArray.buffer && UTF8Decoder) {
        return UTF8Decoder.decode(heapOrArray.subarray(idx, endPtr));
      }
      var str = '';
      // If building with TextDecoder, we have already computed the string length
      // above, so test loop end condition against that
      while (idx < endPtr) {
        // For UTF8 byte structure, see:
        // http://en.wikipedia.org/wiki/UTF-8#Description
        // https://www.ietf.org/rfc/rfc2279.txt
        // https://tools.ietf.org/html/rfc3629
        var u0 = heapOrArray[idx++];
        if (!(u0 & 0x80)) { str += String.fromCharCode(u0); continue; }
        var u1 = heapOrArray[idx++] & 63;
        if ((u0 & 0xE0) == 0xC0) { str += String.fromCharCode(((u0 & 31) << 6) | u1); continue; }
        var u2 = heapOrArray[idx++] & 63;
        if ((u0 & 0xF0) == 0xE0) {
          u0 = ((u0 & 15) << 12) | (u1 << 6) | u2;
        } else {
          if ((u0 & 0xF8) != 0xF0) warnOnce('Invalid UTF-8 leading byte ' + ptrToString(u0) + ' encountered when deserializing a UTF-8 string in wasm memory to a JS string!');
          u0 = ((u0 & 7) << 18) | (u1 << 12) | (u2 << 6) | (heapOrArray[idx++] & 63);
        }
  
        if (u0 < 0x10000) {
          str += String.fromCharCode(u0);
        } else {
          var ch = u0 - 0x10000;
          str += String.fromCharCode(0xD800 | (ch >> 10), 0xDC00 | (ch & 0x3FF));
        }
      }
      return str;
    };
  
    /**
     * Given a pointer 'ptr' to a null-terminated UTF8-encoded string in the
     * emscripten HEAP, returns a copy of that string as a Javascript String object.
     *
     * @param {number} ptr
     * @param {number=} maxBytesToRead - An optional length that specifies the
     *   maximum number of bytes to read. You can omit this parameter to scan the
     *   string until the first 0 byte. If maxBytesToRead is passed, and the string
     *   at [ptr, ptr+maxBytesToReadr[ contains a null byte in the middle, then the
     *   string will cut short at that byte index (i.e. maxBytesToRead will not
     *   produce a string of exact length [ptr, ptr+maxBytesToRead[) N.B. mixing
     *   frequent uses of UTF8ToString() with and without maxBytesToRead may throw
     *   JS JIT optimizations off, so it is worth to consider consistently using one
     * @return {string}
     */
  var UTF8ToString = (ptr, maxBytesToRead) => {
      assert(typeof ptr == 'number', `UTF8ToString expects a number (got ${typeof ptr})`);
      return ptr ? UTF8ArrayToString(HEAPU8, ptr, maxBytesToRead) : '';
    };
  var ___assert_fail = (condition, filename, line, func) =>
      abort(`Assertion failed: ${UTF8ToString(condition)}, at: ` + [filename ? UTF8ToString(filename) : 'unknown filename', line, func ? UTF8ToString(func) : 'unknown function']);

  
  
  var ___handle_stack_overflow = (requested) => {
      var base = _emscripten_stack_get_base();
      var end = _emscripten_stack_get_end();
      abort(`stack overflow (Attempt to set SP to ${ptrToString(requested)}` +
            `, with stack limits [${ptrToString(end)} - ${ptrToString(base)}` +
            ']). If you require more stack space build with -sSTACK_SIZE=<bytes>');
    };

  var PATH = {
  isAbs:(path) => path.charAt(0) === '/',
  splitPath:(filename) => {
        var splitPathRe = /^(\/?|)([\s\S]*?)((?:\.{1,2}|[^\/]+?|)(\.[^.\/]*|))(?:[\/]*)$/;
        return splitPathRe.exec(filename).slice(1);
      },
  normalizeArray:(parts, allowAboveRoot) => {
        // if the path tries to go above the root, `up` ends up > 0
        var up = 0;
        for (var i = parts.length - 1; i >= 0; i--) {
          var last = parts[i];
          if (last === '.') {
            parts.splice(i, 1);
          } else if (last === '..') {
            parts.splice(i, 1);
            up++;
          } else if (up) {
            parts.splice(i, 1);
            up--;
          }
        }
        // if the path is allowed to go above the root, restore leading ..s
        if (allowAboveRoot) {
          for (; up; up--) {
            parts.unshift('..');
          }
        }
        return parts;
      },
  normalize:(path) => {
        var isAbsolute = PATH.isAbs(path),
            trailingSlash = path.substr(-1) === '/';
        // Normalize the path
        path = PATH.normalizeArray(path.split('/').filter((p) => !!p), !isAbsolute).join('/');
        if (!path && !isAbsolute) {
          path = '.';
        }
        if (path && trailingSlash) {
          path += '/';
        }
        return (isAbsolute ? '/' : '') + path;
      },
  dirname:(path) => {
        var result = PATH.splitPath(path),
            root = result[0],
            dir = result[1];
        if (!root && !dir) {
          // No dirname whatsoever
          return '.';
        }
        if (dir) {
          // It has a dirname, strip trailing slash
          dir = dir.substr(0, dir.length - 1);
        }
        return root + dir;
      },
  basename:(path) => {
        // EMSCRIPTEN return '/'' for '/', not an empty string
        if (path === '/') return '/';
        path = PATH.normalize(path);
        path = path.replace(/\/$/, "");
        var lastSlash = path.lastIndexOf('/');
        if (lastSlash === -1) return path;
        return path.substr(lastSlash+1);
      },
  join:(...paths) => PATH.normalize(paths.join('/')),
  join2:(l, r) => PATH.normalize(l + '/' + r),
  };
  
  var initRandomFill = () => {
      if (typeof crypto == 'object' && typeof crypto['getRandomValues'] == 'function') {
        // for modern web browsers
        return (view) => crypto.getRandomValues(view);
      } else
      // we couldn't find a proper implementation, as Math.random() is not suitable for /dev/random, see emscripten-core/emscripten/pull/7096
      abort('no cryptographic support found for randomDevice. consider polyfilling it if you want to use something insecure like Math.random(), e.g. put this in a --pre-js: var crypto = { getRandomValues: (array) => { for (var i = 0; i < array.length; i++) array[i] = (Math.random()*256)|0 } };');
    };
  var randomFill = (view) => {
      // Lazily init on the first invocation.
      return (randomFill = initRandomFill())(view);
    };
  
  
  
  var PATH_FS = {
  resolve:(...args) => {
        var resolvedPath = '',
          resolvedAbsolute = false;
        for (var i = args.length - 1; i >= -1 && !resolvedAbsolute; i--) {
          var path = (i >= 0) ? args[i] : FS.cwd();
          // Skip empty and invalid entries
          if (typeof path != 'string') {
            throw new TypeError('Arguments to path.resolve must be strings');
          } else if (!path) {
            return ''; // an invalid portion invalidates the whole thing
          }
          resolvedPath = path + '/' + resolvedPath;
          resolvedAbsolute = PATH.isAbs(path);
        }
        // At this point the path should be resolved to a full absolute path, but
        // handle relative paths to be safe (might happen when process.cwd() fails)
        resolvedPath = PATH.normalizeArray(resolvedPath.split('/').filter((p) => !!p), !resolvedAbsolute).join('/');
        return ((resolvedAbsolute ? '/' : '') + resolvedPath) || '.';
      },
  relative:(from, to) => {
        from = PATH_FS.resolve(from).substr(1);
        to = PATH_FS.resolve(to).substr(1);
        function trim(arr) {
          var start = 0;
          for (; start < arr.length; start++) {
            if (arr[start] !== '') break;
          }
          var end = arr.length - 1;
          for (; end >= 0; end--) {
            if (arr[end] !== '') break;
          }
          if (start > end) return [];
          return arr.slice(start, end - start + 1);
        }
        var fromParts = trim(from.split('/'));
        var toParts = trim(to.split('/'));
        var length = Math.min(fromParts.length, toParts.length);
        var samePartsLength = length;
        for (var i = 0; i < length; i++) {
          if (fromParts[i] !== toParts[i]) {
            samePartsLength = i;
            break;
          }
        }
        var outputParts = [];
        for (var i = samePartsLength; i < fromParts.length; i++) {
          outputParts.push('..');
        }
        outputParts = outputParts.concat(toParts.slice(samePartsLength));
        return outputParts.join('/');
      },
  };
  
  
  
  var FS_stdin_getChar_buffer = [];
  
  var lengthBytesUTF8 = (str) => {
      var len = 0;
      for (var i = 0; i < str.length; ++i) {
        // Gotcha: charCodeAt returns a 16-bit word that is a UTF-16 encoded code
        // unit, not a Unicode code point of the character! So decode
        // UTF16->UTF32->UTF8.
        // See http://unicode.org/faq/utf_bom.html#utf16-3
        var c = str.charCodeAt(i); // possibly a lead surrogate
        if (c <= 0x7F) {
          len++;
        } else if (c <= 0x7FF) {
          len += 2;
        } else if (c >= 0xD800 && c <= 0xDFFF) {
          len += 4; ++i;
        } else {
          len += 3;
        }
      }
      return len;
    };
  
  var stringToUTF8Array = (str, heap, outIdx, maxBytesToWrite) => {
      assert(typeof str === 'string', `stringToUTF8Array expects a string (got ${typeof str})`);
      // Parameter maxBytesToWrite is not optional. Negative values, 0, null,
      // undefined and false each don't write out any bytes.
      if (!(maxBytesToWrite > 0))
        return 0;
  
      var startIdx = outIdx;
      var endIdx = outIdx + maxBytesToWrite - 1; // -1 for string null terminator.
      for (var i = 0; i < str.length; ++i) {
        // Gotcha: charCodeAt returns a 16-bit word that is a UTF-16 encoded code
        // unit, not a Unicode code point of the character! So decode
        // UTF16->UTF32->UTF8.
        // See http://unicode.org/faq/utf_bom.html#utf16-3
        // For UTF8 byte structure, see http://en.wikipedia.org/wiki/UTF-8#Description
        // and https://www.ietf.org/rfc/rfc2279.txt
        // and https://tools.ietf.org/html/rfc3629
        var u = str.charCodeAt(i); // possibly a lead surrogate
        if (u >= 0xD800 && u <= 0xDFFF) {
          var u1 = str.charCodeAt(++i);
          u = 0x10000 + ((u & 0x3FF) << 10) | (u1 & 0x3FF);
        }
        if (u <= 0x7F) {
          if (outIdx >= endIdx) break;
          heap[outIdx++] = u;
        } else if (u <= 0x7FF) {
          if (outIdx + 1 >= endIdx) break;
          heap[outIdx++] = 0xC0 | (u >> 6);
          heap[outIdx++] = 0x80 | (u & 63);
        } else if (u <= 0xFFFF) {
          if (outIdx + 2 >= endIdx) break;
          heap[outIdx++] = 0xE0 | (u >> 12);
          heap[outIdx++] = 0x80 | ((u >> 6) & 63);
          heap[outIdx++] = 0x80 | (u & 63);
        } else {
          if (outIdx + 3 >= endIdx) break;
          if (u > 0x10FFFF) warnOnce('Invalid Unicode code point ' + ptrToString(u) + ' encountered when serializing a JS string to a UTF-8 string in wasm memory! (Valid unicode code points should be in range 0-0x10FFFF).');
          heap[outIdx++] = 0xF0 | (u >> 18);
          heap[outIdx++] = 0x80 | ((u >> 12) & 63);
          heap[outIdx++] = 0x80 | ((u >> 6) & 63);
          heap[outIdx++] = 0x80 | (u & 63);
        }
      }
      // Null-terminate the pointer to the buffer.
      heap[outIdx] = 0;
      return outIdx - startIdx;
    };
  /** @type {function(string, boolean=, number=)} */
  function intArrayFromString(stringy, dontAddNull, length) {
    var len = length > 0 ? length : lengthBytesUTF8(stringy)+1;
    var u8array = new Array(len);
    var numBytesWritten = stringToUTF8Array(stringy, u8array, 0, u8array.length);
    if (dontAddNull) u8array.length = numBytesWritten;
    return u8array;
  }
  var FS_stdin_getChar = () => {
      if (!FS_stdin_getChar_buffer.length) {
        var result = null;
        if (typeof window != 'undefined' &&
          typeof window.prompt == 'function') {
          // Browser.
          result = window.prompt('Input: ');  // returns null on cancel
          if (result !== null) {
            result += '\n';
          }
        } else
        {}
        if (!result) {
          return null;
        }
        FS_stdin_getChar_buffer = intArrayFromString(result, true);
      }
      return FS_stdin_getChar_buffer.shift();
    };
  var TTY = {
  ttys:[],
  init() {
        // https://github.com/emscripten-core/emscripten/pull/1555
        // if (ENVIRONMENT_IS_NODE) {
        //   // currently, FS.init does not distinguish if process.stdin is a file or TTY
        //   // device, it always assumes it's a TTY device. because of this, we're forcing
        //   // process.stdin to UTF8 encoding to at least make stdin reading compatible
        //   // with text files until FS.init can be refactored.
        //   process.stdin.setEncoding('utf8');
        // }
      },
  shutdown() {
        // https://github.com/emscripten-core/emscripten/pull/1555
        // if (ENVIRONMENT_IS_NODE) {
        //   // inolen: any idea as to why node -e 'process.stdin.read()' wouldn't exit immediately (with process.stdin being a tty)?
        //   // isaacs: because now it's reading from the stream, you've expressed interest in it, so that read() kicks off a _read() which creates a ReadReq operation
        //   // inolen: I thought read() in that case was a synchronous operation that just grabbed some amount of buffered data if it exists?
        //   // isaacs: it is. but it also triggers a _read() call, which calls readStart() on the handle
        //   // isaacs: do process.stdin.pause() and i'd think it'd probably close the pending call
        //   process.stdin.pause();
        // }
      },
  register(dev, ops) {
        TTY.ttys[dev] = { input: [], output: [], ops: ops };
        FS.registerDevice(dev, TTY.stream_ops);
      },
  stream_ops:{
  open(stream) {
          var tty = TTY.ttys[stream.node.rdev];
          if (!tty) {
            throw new FS.ErrnoError(43);
          }
          stream.tty = tty;
          stream.seekable = false;
        },
  close(stream) {
          // flush any pending line data
          stream.tty.ops.fsync(stream.tty);
        },
  fsync(stream) {
          stream.tty.ops.fsync(stream.tty);
        },
  read(stream, buffer, offset, length, pos /* ignored */) {
          if (!stream.tty || !stream.tty.ops.get_char) {
            throw new FS.ErrnoError(60);
          }
          var bytesRead = 0;
          for (var i = 0; i < length; i++) {
            var result;
            try {
              result = stream.tty.ops.get_char(stream.tty);
            } catch (e) {
              throw new FS.ErrnoError(29);
            }
            if (result === undefined && bytesRead === 0) {
              throw new FS.ErrnoError(6);
            }
            if (result === null || result === undefined) break;
            bytesRead++;
            buffer[offset+i] = result;
          }
          if (bytesRead) {
            stream.node.atime = Date.now();
          }
          return bytesRead;
        },
  write(stream, buffer, offset, length, pos) {
          if (!stream.tty || !stream.tty.ops.put_char) {
            throw new FS.ErrnoError(60);
          }
          try {
            for (var i = 0; i < length; i++) {
              stream.tty.ops.put_char(stream.tty, buffer[offset+i]);
            }
          } catch (e) {
            throw new FS.ErrnoError(29);
          }
          if (length) {
            stream.node.mtime = stream.node.ctime = Date.now();
          }
          return i;
        },
  },
  default_tty_ops:{
  get_char(tty) {
          return FS_stdin_getChar();
        },
  put_char(tty, val) {
          if (val === null || val === 10) {
            out(UTF8ArrayToString(tty.output));
            tty.output = [];
          } else {
            if (val != 0) tty.output.push(val); // val == 0 would cut text output off in the middle.
          }
        },
  fsync(tty) {
          if (tty.output && tty.output.length > 0) {
            out(UTF8ArrayToString(tty.output));
            tty.output = [];
          }
        },
  ioctl_tcgets(tty) {
          // typical setting
          return {
            c_iflag: 25856,
            c_oflag: 5,
            c_cflag: 191,
            c_lflag: 35387,
            c_cc: [
              0x03, 0x1c, 0x7f, 0x15, 0x04, 0x00, 0x01, 0x00, 0x11, 0x13, 0x1a, 0x00,
              0x12, 0x0f, 0x17, 0x16, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
              0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            ]
          };
        },
  ioctl_tcsets(tty, optional_actions, data) {
          // currently just ignore
          return 0;
        },
  ioctl_tiocgwinsz(tty) {
          return [24, 80];
        },
  },
  default_tty1_ops:{
  put_char(tty, val) {
          if (val === null || val === 10) {
            err(UTF8ArrayToString(tty.output));
            tty.output = [];
          } else {
            if (val != 0) tty.output.push(val);
          }
        },
  fsync(tty) {
          if (tty.output && tty.output.length > 0) {
            err(UTF8ArrayToString(tty.output));
            tty.output = [];
          }
        },
  },
  };
  
  
  var zeroMemory = (address, size) => {
      HEAPU8.fill(0, address, address + size);
    };
  
  var alignMemory = (size, alignment) => {
      assert(alignment, "alignment argument is required");
      return Math.ceil(size / alignment) * alignment;
    };
  var mmapAlloc = (size) => {
      size = alignMemory(size, 65536);
      var ptr = _emscripten_builtin_memalign(65536, size);
      if (ptr) zeroMemory(ptr, size);
      return ptr;
    };
  var MEMFS = {
  ops_table:null,
  mount(mount) {
        return MEMFS.createNode(null, '/', 16895, 0);
      },
  createNode(parent, name, mode, dev) {
        if (FS.isBlkdev(mode) || FS.isFIFO(mode)) {
          // no supported
          throw new FS.ErrnoError(63);
        }
        MEMFS.ops_table ||= {
          dir: {
            node: {
              getattr: MEMFS.node_ops.getattr,
              setattr: MEMFS.node_ops.setattr,
              lookup: MEMFS.node_ops.lookup,
              mknod: MEMFS.node_ops.mknod,
              rename: MEMFS.node_ops.rename,
              unlink: MEMFS.node_ops.unlink,
              rmdir: MEMFS.node_ops.rmdir,
              readdir: MEMFS.node_ops.readdir,
              symlink: MEMFS.node_ops.symlink
            },
            stream: {
              llseek: MEMFS.stream_ops.llseek
            }
          },
          file: {
            node: {
              getattr: MEMFS.node_ops.getattr,
              setattr: MEMFS.node_ops.setattr
            },
            stream: {
              llseek: MEMFS.stream_ops.llseek,
              read: MEMFS.stream_ops.read,
              write: MEMFS.stream_ops.write,
              allocate: MEMFS.stream_ops.allocate,
              mmap: MEMFS.stream_ops.mmap,
              msync: MEMFS.stream_ops.msync
            }
          },
          link: {
            node: {
              getattr: MEMFS.node_ops.getattr,
              setattr: MEMFS.node_ops.setattr,
              readlink: MEMFS.node_ops.readlink
            },
            stream: {}
          },
          chrdev: {
            node: {
              getattr: MEMFS.node_ops.getattr,
              setattr: MEMFS.node_ops.setattr
            },
            stream: FS.chrdev_stream_ops
          }
        };
        var node = FS.createNode(parent, name, mode, dev);
        if (FS.isDir(node.mode)) {
          node.node_ops = MEMFS.ops_table.dir.node;
          node.stream_ops = MEMFS.ops_table.dir.stream;
          node.contents = {};
        } else if (FS.isFile(node.mode)) {
          node.node_ops = MEMFS.ops_table.file.node;
          node.stream_ops = MEMFS.ops_table.file.stream;
          node.usedBytes = 0; // The actual number of bytes used in the typed array, as opposed to contents.length which gives the whole capacity.
          // When the byte data of the file is populated, this will point to either a typed array, or a normal JS array. Typed arrays are preferred
          // for performance, and used by default. However, typed arrays are not resizable like normal JS arrays are, so there is a small disk size
          // penalty involved for appending file writes that continuously grow a file similar to std::vector capacity vs used -scheme.
          node.contents = null; 
        } else if (FS.isLink(node.mode)) {
          node.node_ops = MEMFS.ops_table.link.node;
          node.stream_ops = MEMFS.ops_table.link.stream;
        } else if (FS.isChrdev(node.mode)) {
          node.node_ops = MEMFS.ops_table.chrdev.node;
          node.stream_ops = MEMFS.ops_table.chrdev.stream;
        }
        node.atime = node.mtime = node.ctime = Date.now();
        // add the new node to the parent
        if (parent) {
          parent.contents[name] = node;
          parent.atime = parent.mtime = parent.ctime = node.atime;
        }
        return node;
      },
  getFileDataAsTypedArray(node) {
        if (!node.contents) return new Uint8Array(0);
        if (node.contents.subarray) return node.contents.subarray(0, node.usedBytes); // Make sure to not return excess unused bytes.
        return new Uint8Array(node.contents);
      },
  expandFileStorage(node, newCapacity) {
        var prevCapacity = node.contents ? node.contents.length : 0;
        if (prevCapacity >= newCapacity) return; // No need to expand, the storage was already large enough.
        // Don't expand strictly to the given requested limit if it's only a very small increase, but instead geometrically grow capacity.
        // For small filesizes (<1MB), perform size*2 geometric increase, but for large sizes, do a much more conservative size*1.125 increase to
        // avoid overshooting the allocation cap by a very large margin.
        var CAPACITY_DOUBLING_MAX = 1024 * 1024;
        newCapacity = Math.max(newCapacity, (prevCapacity * (prevCapacity < CAPACITY_DOUBLING_MAX ? 2.0 : 1.125)) >>> 0);
        if (prevCapacity != 0) newCapacity = Math.max(newCapacity, 256); // At minimum allocate 256b for each file when expanding.
        var oldContents = node.contents;
        node.contents = new Uint8Array(newCapacity); // Allocate new storage.
        if (node.usedBytes > 0) node.contents.set(oldContents.subarray(0, node.usedBytes), 0); // Copy old data over to the new storage.
      },
  resizeFileStorage(node, newSize) {
        if (node.usedBytes == newSize) return;
        if (newSize == 0) {
          node.contents = null; // Fully decommit when requesting a resize to zero.
          node.usedBytes = 0;
        } else {
          var oldContents = node.contents;
          node.contents = new Uint8Array(newSize); // Allocate new storage.
          if (oldContents) {
            node.contents.set(oldContents.subarray(0, Math.min(newSize, node.usedBytes))); // Copy old data over to the new storage.
          }
          node.usedBytes = newSize;
        }
      },
  node_ops:{
  getattr(node) {
          var attr = {};
          // device numbers reuse inode numbers.
          attr.dev = FS.isChrdev(node.mode) ? node.id : 1;
          attr.ino = node.id;
          attr.mode = node.mode;
          attr.nlink = 1;
          attr.uid = 0;
          attr.gid = 0;
          attr.rdev = node.rdev;
          if (FS.isDir(node.mode)) {
            attr.size = 4096;
          } else if (FS.isFile(node.mode)) {
            attr.size = node.usedBytes;
          } else if (FS.isLink(node.mode)) {
            attr.size = node.link.length;
          } else {
            attr.size = 0;
          }
          attr.atime = new Date(node.atime);
          attr.mtime = new Date(node.mtime);
          attr.ctime = new Date(node.ctime);
          // NOTE: In our implementation, st_blocks = Math.ceil(st_size/st_blksize),
          //       but this is not required by the standard.
          attr.blksize = 4096;
          attr.blocks = Math.ceil(attr.size / attr.blksize);
          return attr;
        },
  setattr(node, attr) {
          for (const key of ["mode", "atime", "mtime", "ctime"]) {
            if (attr[key]) {
              node[key] = attr[key];
            }
          }
          if (attr.size !== undefined) {
            MEMFS.resizeFileStorage(node, attr.size);
          }
        },
  lookup(parent, name) {
          throw new FS.ErrnoError(44);
        },
  mknod(parent, name, mode, dev) {
          return MEMFS.createNode(parent, name, mode, dev);
        },
  rename(old_node, new_dir, new_name) {
          var new_node;
          try {
            new_node = FS.lookupNode(new_dir, new_name);
          } catch (e) {}
          if (new_node) {
            if (FS.isDir(old_node.mode)) {
              // if we're overwriting a directory at new_name, make sure it's empty.
              for (var i in new_node.contents) {
                throw new FS.ErrnoError(55);
              }
            }
            FS.hashRemoveNode(new_node);
          }
          // do the internal rewiring
          delete old_node.parent.contents[old_node.name];
          new_dir.contents[new_name] = old_node;
          old_node.name = new_name;
          new_dir.ctime = new_dir.mtime = old_node.parent.ctime = old_node.parent.mtime = Date.now();
        },
  unlink(parent, name) {
          delete parent.contents[name];
          parent.ctime = parent.mtime = Date.now();
        },
  rmdir(parent, name) {
          var node = FS.lookupNode(parent, name);
          for (var i in node.contents) {
            throw new FS.ErrnoError(55);
          }
          delete parent.contents[name];
          parent.ctime = parent.mtime = Date.now();
        },
  readdir(node) {
          return ['.', '..', ...Object.keys(node.contents)];
        },
  symlink(parent, newname, oldpath) {
          var node = MEMFS.createNode(parent, newname, 0o777 | 40960, 0);
          node.link = oldpath;
          return node;
        },
  readlink(node) {
          if (!FS.isLink(node.mode)) {
            throw new FS.ErrnoError(28);
          }
          return node.link;
        },
  },
  stream_ops:{
  read(stream, buffer, offset, length, position) {
          var contents = stream.node.contents;
          if (position >= stream.node.usedBytes) return 0;
          var size = Math.min(stream.node.usedBytes - position, length);
          assert(size >= 0);
          if (size > 8 && contents.subarray) { // non-trivial, and typed array
            buffer.set(contents.subarray(position, position + size), offset);
          } else {
            for (var i = 0; i < size; i++) buffer[offset + i] = contents[position + i];
          }
          return size;
        },
  write(stream, buffer, offset, length, position, canOwn) {
          // The data buffer should be a typed array view
          assert(!(buffer instanceof ArrayBuffer));
          // If the buffer is located in main memory (HEAP), and if
          // memory can grow, we can't hold on to references of the
          // memory buffer, as they may get invalidated. That means we
          // need to do copy its contents.
          if (buffer.buffer === HEAP8.buffer) {
            canOwn = false;
          }
  
          if (!length) return 0;
          var node = stream.node;
          node.mtime = node.ctime = Date.now();
  
          if (buffer.subarray && (!node.contents || node.contents.subarray)) { // This write is from a typed array to a typed array?
            if (canOwn) {
              assert(position === 0, 'canOwn must imply no weird position inside the file');
              node.contents = buffer.subarray(offset, offset + length);
              node.usedBytes = length;
              return length;
            } else if (node.usedBytes === 0 && position === 0) { // If this is a simple first write to an empty file, do a fast set since we don't need to care about old data.
              node.contents = buffer.slice(offset, offset + length);
              node.usedBytes = length;
              return length;
            } else if (position + length <= node.usedBytes) { // Writing to an already allocated and used subrange of the file?
              node.contents.set(buffer.subarray(offset, offset + length), position);
              return length;
            }
          }
  
          // Appending to an existing file and we need to reallocate, or source data did not come as a typed array.
          MEMFS.expandFileStorage(node, position+length);
          if (node.contents.subarray && buffer.subarray) {
            // Use typed array write which is available.
            node.contents.set(buffer.subarray(offset, offset + length), position);
          } else {
            for (var i = 0; i < length; i++) {
             node.contents[position + i] = buffer[offset + i]; // Or fall back to manual write if not.
            }
          }
          node.usedBytes = Math.max(node.usedBytes, position + length);
          return length;
        },
  llseek(stream, offset, whence) {
          var position = offset;
          if (whence === 1) {
            position += stream.position;
          } else if (whence === 2) {
            if (FS.isFile(stream.node.mode)) {
              position += stream.node.usedBytes;
            }
          }
          if (position < 0) {
            throw new FS.ErrnoError(28);
          }
          return position;
        },
  allocate(stream, offset, length) {
          MEMFS.expandFileStorage(stream.node, offset + length);
          stream.node.usedBytes = Math.max(stream.node.usedBytes, offset + length);
        },
  mmap(stream, length, position, prot, flags) {
          if (!FS.isFile(stream.node.mode)) {
            throw new FS.ErrnoError(43);
          }
          var ptr;
          var allocated;
          var contents = stream.node.contents;
          // Only make a new copy when MAP_PRIVATE is specified.
          if (!(flags & 2) && contents && contents.buffer === HEAP8.buffer) {
            // We can't emulate MAP_SHARED when the file is not backed by the
            // buffer we're mapping to (e.g. the HEAP buffer).
            allocated = false;
            ptr = contents.byteOffset;
          } else {
            allocated = true;
            ptr = mmapAlloc(length);
            if (!ptr) {
              throw new FS.ErrnoError(48);
            }
            if (contents) {
              // Try to avoid unnecessary slices.
              if (position > 0 || position + length < contents.length) {
                if (contents.subarray) {
                  contents = contents.subarray(position, position + length);
                } else {
                  contents = Array.prototype.slice.call(contents, position, position + length);
                }
              }
              HEAP8.set(contents, ptr);
            }
          }
          return { ptr, allocated };
        },
  msync(stream, buffer, offset, length, mmapFlags) {
          MEMFS.stream_ops.write(stream, buffer, 0, length, offset, false);
          // should we check if bytesWritten and length are the same?
          return 0;
        },
  },
  };
  
  var asyncLoad = async (url) => {
      var arrayBuffer = await readAsync(url);
      assert(arrayBuffer, `Loading data file "${url}" failed (no arrayBuffer).`);
      return new Uint8Array(arrayBuffer);
    };
  
  
  var FS_createDataFile = (parent, name, fileData, canRead, canWrite, canOwn) => {
      FS.createDataFile(parent, name, fileData, canRead, canWrite, canOwn);
    };
  
  var preloadPlugins = Module['preloadPlugins'] || [];
  var FS_handledByPreloadPlugin = (byteArray, fullname, finish, onerror) => {
      // Ensure plugins are ready.
      if (typeof Browser != 'undefined') Browser.init();
  
      var handled = false;
      preloadPlugins.forEach((plugin) => {
        if (handled) return;
        if (plugin['canHandle'](fullname)) {
          plugin['handle'](byteArray, fullname, finish, onerror);
          handled = true;
        }
      });
      return handled;
    };
  var FS_createPreloadedFile = (parent, name, url, canRead, canWrite, onload, onerror, dontCreateFile, canOwn, preFinish) => {
      // TODO we should allow people to just pass in a complete filename instead
      // of parent and name being that we just join them anyways
      var fullname = name ? PATH_FS.resolve(PATH.join2(parent, name)) : parent;
      var dep = getUniqueRunDependency(`cp ${fullname}`); // might have several active requests for the same fullname
      function processData(byteArray) {
        function finish(byteArray) {
          preFinish?.();
          if (!dontCreateFile) {
            FS_createDataFile(parent, name, byteArray, canRead, canWrite, canOwn);
          }
          onload?.();
          removeRunDependency(dep);
        }
        if (FS_handledByPreloadPlugin(byteArray, fullname, finish, () => {
          onerror?.();
          removeRunDependency(dep);
        })) {
          return;
        }
        finish(byteArray);
      }
      addRunDependency(dep);
      if (typeof url == 'string') {
        asyncLoad(url).then(processData, onerror);
      } else {
        processData(url);
      }
    };
  
  var FS_modeStringToFlags = (str) => {
      var flagModes = {
        'r': 0,
        'r+': 2,
        'w': 512 | 64 | 1,
        'w+': 512 | 64 | 2,
        'a': 1024 | 64 | 1,
        'a+': 1024 | 64 | 2,
      };
      var flags = flagModes[str];
      if (typeof flags == 'undefined') {
        throw new Error(`Unknown file open mode: ${str}`);
      }
      return flags;
    };
  
  var FS_getMode = (canRead, canWrite) => {
      var mode = 0;
      if (canRead) mode |= 292 | 73;
      if (canWrite) mode |= 146;
      return mode;
    };
  
  
  
  
  
  
  var strError = (errno) => UTF8ToString(_strerror(errno));
  
  var ERRNO_CODES = {
      'EPERM': 63,
      'ENOENT': 44,
      'ESRCH': 71,
      'EINTR': 27,
      'EIO': 29,
      'ENXIO': 60,
      'E2BIG': 1,
      'ENOEXEC': 45,
      'EBADF': 8,
      'ECHILD': 12,
      'EAGAIN': 6,
      'EWOULDBLOCK': 6,
      'ENOMEM': 48,
      'EACCES': 2,
      'EFAULT': 21,
      'ENOTBLK': 105,
      'EBUSY': 10,
      'EEXIST': 20,
      'EXDEV': 75,
      'ENODEV': 43,
      'ENOTDIR': 54,
      'EISDIR': 31,
      'EINVAL': 28,
      'ENFILE': 41,
      'EMFILE': 33,
      'ENOTTY': 59,
      'ETXTBSY': 74,
      'EFBIG': 22,
      'ENOSPC': 51,
      'ESPIPE': 70,
      'EROFS': 69,
      'EMLINK': 34,
      'EPIPE': 64,
      'EDOM': 18,
      'ERANGE': 68,
      'ENOMSG': 49,
      'EIDRM': 24,
      'ECHRNG': 106,
      'EL2NSYNC': 156,
      'EL3HLT': 107,
      'EL3RST': 108,
      'ELNRNG': 109,
      'EUNATCH': 110,
      'ENOCSI': 111,
      'EL2HLT': 112,
      'EDEADLK': 16,
      'ENOLCK': 46,
      'EBADE': 113,
      'EBADR': 114,
      'EXFULL': 115,
      'ENOANO': 104,
      'EBADRQC': 103,
      'EBADSLT': 102,
      'EDEADLOCK': 16,
      'EBFONT': 101,
      'ENOSTR': 100,
      'ENODATA': 116,
      'ETIME': 117,
      'ENOSR': 118,
      'ENONET': 119,
      'ENOPKG': 120,
      'EREMOTE': 121,
      'ENOLINK': 47,
      'EADV': 122,
      'ESRMNT': 123,
      'ECOMM': 124,
      'EPROTO': 65,
      'EMULTIHOP': 36,
      'EDOTDOT': 125,
      'EBADMSG': 9,
      'ENOTUNIQ': 126,
      'EBADFD': 127,
      'EREMCHG': 128,
      'ELIBACC': 129,
      'ELIBBAD': 130,
      'ELIBSCN': 131,
      'ELIBMAX': 132,
      'ELIBEXEC': 133,
      'ENOSYS': 52,
      'ENOTEMPTY': 55,
      'ENAMETOOLONG': 37,
      'ELOOP': 32,
      'EOPNOTSUPP': 138,
      'EPFNOSUPPORT': 139,
      'ECONNRESET': 15,
      'ENOBUFS': 42,
      'EAFNOSUPPORT': 5,
      'EPROTOTYPE': 67,
      'ENOTSOCK': 57,
      'ENOPROTOOPT': 50,
      'ESHUTDOWN': 140,
      'ECONNREFUSED': 14,
      'EADDRINUSE': 3,
      'ECONNABORTED': 13,
      'ENETUNREACH': 40,
      'ENETDOWN': 38,
      'ETIMEDOUT': 73,
      'EHOSTDOWN': 142,
      'EHOSTUNREACH': 23,
      'EINPROGRESS': 26,
      'EALREADY': 7,
      'EDESTADDRREQ': 17,
      'EMSGSIZE': 35,
      'EPROTONOSUPPORT': 66,
      'ESOCKTNOSUPPORT': 137,
      'EADDRNOTAVAIL': 4,
      'ENETRESET': 39,
      'EISCONN': 30,
      'ENOTCONN': 53,
      'ETOOMANYREFS': 141,
      'EUSERS': 136,
      'EDQUOT': 19,
      'ESTALE': 72,
      'ENOTSUP': 138,
      'ENOMEDIUM': 148,
      'EILSEQ': 25,
      'EOVERFLOW': 61,
      'ECANCELED': 11,
      'ENOTRECOVERABLE': 56,
      'EOWNERDEAD': 62,
      'ESTRPIPE': 135,
    };
  var FS = {
  root:null,
  mounts:[],
  devices:{
  },
  streams:[],
  nextInode:1,
  nameTable:null,
  currentPath:"/",
  initialized:false,
  ignorePermissions:true,
  ErrnoError:class extends Error {
        name = 'ErrnoError';
        // We set the `name` property to be able to identify `FS.ErrnoError`
        // - the `name` is a standard ECMA-262 property of error objects. Kind of good to have it anyway.
        // - when using PROXYFS, an error can come from an underlying FS
        // as different FS objects have their own FS.ErrnoError each,
        // the test `err instanceof FS.ErrnoError` won't detect an error coming from another filesystem, causing bugs.
        // we'll use the reliable test `err.name == "ErrnoError"` instead
        constructor(errno) {
          super(runtimeInitialized ? strError(errno) : '');
          this.errno = errno;
          for (var key in ERRNO_CODES) {
            if (ERRNO_CODES[key] === errno) {
              this.code = key;
              break;
            }
          }
        }
      },
  filesystems:null,
  syncFSRequests:0,
  readFiles:{
  },
  FSStream:class {
        shared = {};
        get object() {
          return this.node;
        }
        set object(val) {
          this.node = val;
        }
        get isRead() {
          return (this.flags & 2097155) !== 1;
        }
        get isWrite() {
          return (this.flags & 2097155) !== 0;
        }
        get isAppend() {
          return (this.flags & 1024);
        }
        get flags() {
          return this.shared.flags;
        }
        set flags(val) {
          this.shared.flags = val;
        }
        get position() {
          return this.shared.position;
        }
        set position(val) {
          this.shared.position = val;
        }
      },
  FSNode:class {
        node_ops = {};
        stream_ops = {};
        readMode = 292 | 73;
        writeMode = 146;
        mounted = null;
        constructor(parent, name, mode, rdev) {
          if (!parent) {
            parent = this;  // root node sets parent to itself
          }
          this.parent = parent;
          this.mount = parent.mount;
          this.id = FS.nextInode++;
          this.name = name;
          this.mode = mode;
          this.rdev = rdev;
          this.atime = this.mtime = this.ctime = Date.now();
        }
        get read() {
          return (this.mode & this.readMode) === this.readMode;
        }
        set read(val) {
          val ? this.mode |= this.readMode : this.mode &= ~this.readMode;
        }
        get write() {
          return (this.mode & this.writeMode) === this.writeMode;
        }
        set write(val) {
          val ? this.mode |= this.writeMode : this.mode &= ~this.writeMode;
        }
        get isFolder() {
          return FS.isDir(this.mode);
        }
        get isDevice() {
          return FS.isChrdev(this.mode);
        }
      },
  lookupPath(path, opts = {}) {
        if (!path) return { path: '', node: null };
        opts.follow_mount ??= true
  
        if (!PATH.isAbs(path)) {
          path = FS.cwd() + '/' + path;
        }
  
        // limit max consecutive symlinks to 40 (SYMLOOP_MAX).
        linkloop: for (var nlinks = 0; nlinks < 40; nlinks++) {
          // split the absolute path
          var parts = path.split('/').filter((p) => !!p && (p !== '.'));
  
          // start at the root
          var current = FS.root;
          var current_path = '/';
  
          for (var i = 0; i < parts.length; i++) {
            var islast = (i === parts.length-1);
            if (islast && opts.parent) {
              // stop resolving
              break;
            }
  
            if (parts[i] === '..') {
              current_path = PATH.dirname(current_path);
              current = current.parent;
              continue;
            }
  
            current_path = PATH.join2(current_path, parts[i]);
            try {
              current = FS.lookupNode(current, parts[i]);
            } catch (e) {
              // if noent_okay is true, suppress a ENOENT in the last component
              // and return an object with an undefined node. This is needed for
              // resolving symlinks in the path when creating a file.
              if ((e?.errno === 44) && islast && opts.noent_okay) {
                return { path: current_path };
              }
              throw e;
            }
  
            // jump to the mount's root node if this is a mountpoint
            if (FS.isMountpoint(current) && (!islast || opts.follow_mount)) {
              current = current.mounted.root;
            }
  
            // by default, lookupPath will not follow a symlink if it is the final path component.
            // setting opts.follow = true will override this behavior.
            if (FS.isLink(current.mode) && (!islast || opts.follow)) {
              if (!current.node_ops.readlink) {
                throw new FS.ErrnoError(52);
              }
              var link = current.node_ops.readlink(current);
              if (!PATH.isAbs(link)) {
                link = PATH.dirname(current_path) + '/' + link;
              }
              path = link + '/' + parts.slice(i + 1).join('/');
              continue linkloop;
            }
          }
          return { path: current_path, node: current };
        }
        throw new FS.ErrnoError(32);
      },
  getPath(node) {
        var path;
        while (true) {
          if (FS.isRoot(node)) {
            var mount = node.mount.mountpoint;
            if (!path) return mount;
            return mount[mount.length-1] !== '/' ? `${mount}/${path}` : mount + path;
          }
          path = path ? `${node.name}/${path}` : node.name;
          node = node.parent;
        }
      },
  hashName(parentid, name) {
        var hash = 0;
  
        for (var i = 0; i < name.length; i++) {
          hash = ((hash << 5) - hash + name.charCodeAt(i)) | 0;
        }
        return ((parentid + hash) >>> 0) % FS.nameTable.length;
      },
  hashAddNode(node) {
        var hash = FS.hashName(node.parent.id, node.name);
        node.name_next = FS.nameTable[hash];
        FS.nameTable[hash] = node;
      },
  hashRemoveNode(node) {
        var hash = FS.hashName(node.parent.id, node.name);
        if (FS.nameTable[hash] === node) {
          FS.nameTable[hash] = node.name_next;
        } else {
          var current = FS.nameTable[hash];
          while (current) {
            if (current.name_next === node) {
              current.name_next = node.name_next;
              break;
            }
            current = current.name_next;
          }
        }
      },
  lookupNode(parent, name) {
        var errCode = FS.mayLookup(parent);
        if (errCode) {
          throw new FS.ErrnoError(errCode);
        }
        var hash = FS.hashName(parent.id, name);
        for (var node = FS.nameTable[hash]; node; node = node.name_next) {
          var nodeName = node.name;
          if (node.parent.id === parent.id && nodeName === name) {
            return node;
          }
        }
        // if we failed to find it in the cache, call into the VFS
        return FS.lookup(parent, name);
      },
  createNode(parent, name, mode, rdev) {
        assert(typeof parent == 'object')
        var node = new FS.FSNode(parent, name, mode, rdev);
  
        FS.hashAddNode(node);
  
        return node;
      },
  destroyNode(node) {
        FS.hashRemoveNode(node);
      },
  isRoot(node) {
        return node === node.parent;
      },
  isMountpoint(node) {
        return !!node.mounted;
      },
  isFile(mode) {
        return (mode & 61440) === 32768;
      },
  isDir(mode) {
        return (mode & 61440) === 16384;
      },
  isLink(mode) {
        return (mode & 61440) === 40960;
      },
  isChrdev(mode) {
        return (mode & 61440) === 8192;
      },
  isBlkdev(mode) {
        return (mode & 61440) === 24576;
      },
  isFIFO(mode) {
        return (mode & 61440) === 4096;
      },
  isSocket(mode) {
        return (mode & 49152) === 49152;
      },
  flagsToPermissionString(flag) {
        var perms = ['r', 'w', 'rw'][flag & 3];
        if ((flag & 512)) {
          perms += 'w';
        }
        return perms;
      },
  nodePermissions(node, perms) {
        if (FS.ignorePermissions) {
          return 0;
        }
        // return 0 if any user, group or owner bits are set.
        if (perms.includes('r') && !(node.mode & 292)) {
          return 2;
        } else if (perms.includes('w') && !(node.mode & 146)) {
          return 2;
        } else if (perms.includes('x') && !(node.mode & 73)) {
          return 2;
        }
        return 0;
      },
  mayLookup(dir) {
        if (!FS.isDir(dir.mode)) return 54;
        var errCode = FS.nodePermissions(dir, 'x');
        if (errCode) return errCode;
        if (!dir.node_ops.lookup) return 2;
        return 0;
      },
  mayCreate(dir, name) {
        if (!FS.isDir(dir.mode)) {
          return 54;
        }
        try {
          var node = FS.lookupNode(dir, name);
          return 20;
        } catch (e) {
        }
        return FS.nodePermissions(dir, 'wx');
      },
  mayDelete(dir, name, isdir) {
        var node;
        try {
          node = FS.lookupNode(dir, name);
        } catch (e) {
          return e.errno;
        }
        var errCode = FS.nodePermissions(dir, 'wx');
        if (errCode) {
          return errCode;
        }
        if (isdir) {
          if (!FS.isDir(node.mode)) {
            return 54;
          }
          if (FS.isRoot(node) || FS.getPath(node) === FS.cwd()) {
            return 10;
          }
        } else {
          if (FS.isDir(node.mode)) {
            return 31;
          }
        }
        return 0;
      },
  mayOpen(node, flags) {
        if (!node) {
          return 44;
        }
        if (FS.isLink(node.mode)) {
          return 32;
        } else if (FS.isDir(node.mode)) {
          if (FS.flagsToPermissionString(flags) !== 'r' || // opening for write
              (flags & 512)) { // TODO: check for O_SEARCH? (== search for dir only)
            return 31;
          }
        }
        return FS.nodePermissions(node, FS.flagsToPermissionString(flags));
      },
  MAX_OPEN_FDS:4096,
  nextfd() {
        for (var fd = 0; fd <= FS.MAX_OPEN_FDS; fd++) {
          if (!FS.streams[fd]) {
            return fd;
          }
        }
        throw new FS.ErrnoError(33);
      },
  getStreamChecked(fd) {
        var stream = FS.getStream(fd);
        if (!stream) {
          throw new FS.ErrnoError(8);
        }
        return stream;
      },
  getStream:(fd) => FS.streams[fd],
  createStream(stream, fd = -1) {
        assert(fd >= -1);
  
        // clone it, so we can return an instance of FSStream
        stream = Object.assign(new FS.FSStream(), stream);
        if (fd == -1) {
          fd = FS.nextfd();
        }
        stream.fd = fd;
        FS.streams[fd] = stream;
        return stream;
      },
  closeStream(fd) {
        FS.streams[fd] = null;
      },
  dupStream(origStream, fd = -1) {
        var stream = FS.createStream(origStream, fd);
        stream.stream_ops?.dup?.(stream);
        return stream;
      },
  chrdev_stream_ops:{
  open(stream) {
          var device = FS.getDevice(stream.node.rdev);
          // override node's stream ops with the device's
          stream.stream_ops = device.stream_ops;
          // forward the open call
          stream.stream_ops.open?.(stream);
        },
  llseek() {
          throw new FS.ErrnoError(70);
        },
  },
  major:(dev) => ((dev) >> 8),
  minor:(dev) => ((dev) & 0xff),
  makedev:(ma, mi) => ((ma) << 8 | (mi)),
  registerDevice(dev, ops) {
        FS.devices[dev] = { stream_ops: ops };
      },
  getDevice:(dev) => FS.devices[dev],
  getMounts(mount) {
        var mounts = [];
        var check = [mount];
  
        while (check.length) {
          var m = check.pop();
  
          mounts.push(m);
  
          check.push(...m.mounts);
        }
  
        return mounts;
      },
  syncfs(populate, callback) {
        if (typeof populate == 'function') {
          callback = populate;
          populate = false;
        }
  
        FS.syncFSRequests++;
  
        if (FS.syncFSRequests > 1) {
          err(`warning: ${FS.syncFSRequests} FS.syncfs operations in flight at once, probably just doing extra work`);
        }
  
        var mounts = FS.getMounts(FS.root.mount);
        var completed = 0;
  
        function doCallback(errCode) {
          assert(FS.syncFSRequests > 0);
          FS.syncFSRequests--;
          return callback(errCode);
        }
  
        function done(errCode) {
          if (errCode) {
            if (!done.errored) {
              done.errored = true;
              return doCallback(errCode);
            }
            return;
          }
          if (++completed >= mounts.length) {
            doCallback(null);
          }
        };
  
        // sync all mounts
        mounts.forEach((mount) => {
          if (!mount.type.syncfs) {
            return done(null);
          }
          mount.type.syncfs(mount, populate, done);
        });
      },
  mount(type, opts, mountpoint) {
        if (typeof type == 'string') {
          // The filesystem was not included, and instead we have an error
          // message stored in the variable.
          throw type;
        }
        var root = mountpoint === '/';
        var pseudo = !mountpoint;
        var node;
  
        if (root && FS.root) {
          throw new FS.ErrnoError(10);
        } else if (!root && !pseudo) {
          var lookup = FS.lookupPath(mountpoint, { follow_mount: false });
  
          mountpoint = lookup.path;  // use the absolute path
          node = lookup.node;
  
          if (FS.isMountpoint(node)) {
            throw new FS.ErrnoError(10);
          }
  
          if (!FS.isDir(node.mode)) {
            throw new FS.ErrnoError(54);
          }
        }
  
        var mount = {
          type,
          opts,
          mountpoint,
          mounts: []
        };
  
        // create a root node for the fs
        var mountRoot = type.mount(mount);
        mountRoot.mount = mount;
        mount.root = mountRoot;
  
        if (root) {
          FS.root = mountRoot;
        } else if (node) {
          // set as a mountpoint
          node.mounted = mount;
  
          // add the new mount to the current mount's children
          if (node.mount) {
            node.mount.mounts.push(mount);
          }
        }
  
        return mountRoot;
      },
  unmount(mountpoint) {
        var lookup = FS.lookupPath(mountpoint, { follow_mount: false });
  
        if (!FS.isMountpoint(lookup.node)) {
          throw new FS.ErrnoError(28);
        }
  
        // destroy the nodes for this mount, and all its child mounts
        var node = lookup.node;
        var mount = node.mounted;
        var mounts = FS.getMounts(mount);
  
        Object.keys(FS.nameTable).forEach((hash) => {
          var current = FS.nameTable[hash];
  
          while (current) {
            var next = current.name_next;
  
            if (mounts.includes(current.mount)) {
              FS.destroyNode(current);
            }
  
            current = next;
          }
        });
  
        // no longer a mountpoint
        node.mounted = null;
  
        // remove this mount from the child mounts
        var idx = node.mount.mounts.indexOf(mount);
        assert(idx !== -1);
        node.mount.mounts.splice(idx, 1);
      },
  lookup(parent, name) {
        return parent.node_ops.lookup(parent, name);
      },
  mknod(path, mode, dev) {
        var lookup = FS.lookupPath(path, { parent: true });
        var parent = lookup.node;
        var name = PATH.basename(path);
        if (!name || name === '.' || name === '..') {
          throw new FS.ErrnoError(28);
        }
        var errCode = FS.mayCreate(parent, name);
        if (errCode) {
          throw new FS.ErrnoError(errCode);
        }
        if (!parent.node_ops.mknod) {
          throw new FS.ErrnoError(63);
        }
        return parent.node_ops.mknod(parent, name, mode, dev);
      },
  statfs(path) {
  
        // NOTE: None of the defaults here are true. We're just returning safe and
        //       sane values.
        var rtn = {
          bsize: 4096,
          frsize: 4096,
          blocks: 1e6,
          bfree: 5e5,
          bavail: 5e5,
          files: FS.nextInode,
          ffree: FS.nextInode - 1,
          fsid: 42,
          flags: 2,
          namelen: 255,
        };
  
        var parent = FS.lookupPath(path, {follow: true}).node;
        if (parent?.node_ops.statfs) {
          Object.assign(rtn, parent.node_ops.statfs(parent.mount.opts.root));
        }
        return rtn;
      },
  create(path, mode = 0o666) {
        mode &= 4095;
        mode |= 32768;
        return FS.mknod(path, mode, 0);
      },
  mkdir(path, mode = 0o777) {
        mode &= 511 | 512;
        mode |= 16384;
        return FS.mknod(path, mode, 0);
      },
  mkdirTree(path, mode) {
        var dirs = path.split('/');
        var d = '';
        for (var i = 0; i < dirs.length; ++i) {
          if (!dirs[i]) continue;
          d += '/' + dirs[i];
          try {
            FS.mkdir(d, mode);
          } catch(e) {
            if (e.errno != 20) throw e;
          }
        }
      },
  mkdev(path, mode, dev) {
        if (typeof dev == 'undefined') {
          dev = mode;
          mode = 0o666;
        }
        mode |= 8192;
        return FS.mknod(path, mode, dev);
      },
  symlink(oldpath, newpath) {
        if (!PATH_FS.resolve(oldpath)) {
          throw new FS.ErrnoError(44);
        }
        var lookup = FS.lookupPath(newpath, { parent: true });
        var parent = lookup.node;
        if (!parent) {
          throw new FS.ErrnoError(44);
        }
        var newname = PATH.basename(newpath);
        var errCode = FS.mayCreate(parent, newname);
        if (errCode) {
          throw new FS.ErrnoError(errCode);
        }
        if (!parent.node_ops.symlink) {
          throw new FS.ErrnoError(63);
        }
        return parent.node_ops.symlink(parent, newname, oldpath);
      },
  rename(old_path, new_path) {
        var old_dirname = PATH.dirname(old_path);
        var new_dirname = PATH.dirname(new_path);
        var old_name = PATH.basename(old_path);
        var new_name = PATH.basename(new_path);
        // parents must exist
        var lookup, old_dir, new_dir;
  
        // let the errors from non existent directories percolate up
        lookup = FS.lookupPath(old_path, { parent: true });
        old_dir = lookup.node;
        lookup = FS.lookupPath(new_path, { parent: true });
        new_dir = lookup.node;
  
        if (!old_dir || !new_dir) throw new FS.ErrnoError(44);
        // need to be part of the same mount
        if (old_dir.mount !== new_dir.mount) {
          throw new FS.ErrnoError(75);
        }
        // source must exist
        var old_node = FS.lookupNode(old_dir, old_name);
        // old path should not be an ancestor of the new path
        var relative = PATH_FS.relative(old_path, new_dirname);
        if (relative.charAt(0) !== '.') {
          throw new FS.ErrnoError(28);
        }
        // new path should not be an ancestor of the old path
        relative = PATH_FS.relative(new_path, old_dirname);
        if (relative.charAt(0) !== '.') {
          throw new FS.ErrnoError(55);
        }
        // see if the new path already exists
        var new_node;
        try {
          new_node = FS.lookupNode(new_dir, new_name);
        } catch (e) {
          // not fatal
        }
        // early out if nothing needs to change
        if (old_node === new_node) {
          return;
        }
        // we'll need to delete the old entry
        var isdir = FS.isDir(old_node.mode);
        var errCode = FS.mayDelete(old_dir, old_name, isdir);
        if (errCode) {
          throw new FS.ErrnoError(errCode);
        }
        // need delete permissions if we'll be overwriting.
        // need create permissions if new doesn't already exist.
        errCode = new_node ?
          FS.mayDelete(new_dir, new_name, isdir) :
          FS.mayCreate(new_dir, new_name);
        if (errCode) {
          throw new FS.ErrnoError(errCode);
        }
        if (!old_dir.node_ops.rename) {
          throw new FS.ErrnoError(63);
        }
        if (FS.isMountpoint(old_node) || (new_node && FS.isMountpoint(new_node))) {
          throw new FS.ErrnoError(10);
        }
        // if we are going to change the parent, check write permissions
        if (new_dir !== old_dir) {
          errCode = FS.nodePermissions(old_dir, 'w');
          if (errCode) {
            throw new FS.ErrnoError(errCode);
          }
        }
        // remove the node from the lookup hash
        FS.hashRemoveNode(old_node);
        // do the underlying fs rename
        try {
          old_dir.node_ops.rename(old_node, new_dir, new_name);
          // update old node (we do this here to avoid each backend
          // needing to)
          old_node.parent = new_dir;
        } catch (e) {
          throw e;
        } finally {
          // add the node back to the hash (in case node_ops.rename
          // changed its name)
          FS.hashAddNode(old_node);
        }
      },
  rmdir(path) {
        var lookup = FS.lookupPath(path, { parent: true });
        var parent = lookup.node;
        var name = PATH.basename(path);
        var node = FS.lookupNode(parent, name);
        var errCode = FS.mayDelete(parent, name, true);
        if (errCode) {
          throw new FS.ErrnoError(errCode);
        }
        if (!parent.node_ops.rmdir) {
          throw new FS.ErrnoError(63);
        }
        if (FS.isMountpoint(node)) {
          throw new FS.ErrnoError(10);
        }
        parent.node_ops.rmdir(parent, name);
        FS.destroyNode(node);
      },
  readdir(path) {
        var lookup = FS.lookupPath(path, { follow: true });
        var node = lookup.node;
        if (!node.node_ops.readdir) {
          throw new FS.ErrnoError(54);
        }
        return node.node_ops.readdir(node);
      },
  unlink(path) {
        var lookup = FS.lookupPath(path, { parent: true });
        var parent = lookup.node;
        if (!parent) {
          throw new FS.ErrnoError(44);
        }
        var name = PATH.basename(path);
        var node = FS.lookupNode(parent, name);
        var errCode = FS.mayDelete(parent, name, false);
        if (errCode) {
          // According to POSIX, we should map EISDIR to EPERM, but
          // we instead do what Linux does (and we must, as we use
          // the musl linux libc).
          throw new FS.ErrnoError(errCode);
        }
        if (!parent.node_ops.unlink) {
          throw new FS.ErrnoError(63);
        }
        if (FS.isMountpoint(node)) {
          throw new FS.ErrnoError(10);
        }
        parent.node_ops.unlink(parent, name);
        FS.destroyNode(node);
      },
  readlink(path) {
        var lookup = FS.lookupPath(path);
        var link = lookup.node;
        if (!link) {
          throw new FS.ErrnoError(44);
        }
        if (!link.node_ops.readlink) {
          throw new FS.ErrnoError(28);
        }
        return link.node_ops.readlink(link);
      },
  stat(path, dontFollow) {
        var lookup = FS.lookupPath(path, { follow: !dontFollow });
        var node = lookup.node;
        if (!node) {
          throw new FS.ErrnoError(44);
        }
        if (!node.node_ops.getattr) {
          throw new FS.ErrnoError(63);
        }
        return node.node_ops.getattr(node);
      },
  lstat(path) {
        return FS.stat(path, true);
      },
  chmod(path, mode, dontFollow) {
        var node;
        if (typeof path == 'string') {
          var lookup = FS.lookupPath(path, { follow: !dontFollow });
          node = lookup.node;
        } else {
          node = path;
        }
        if (!node.node_ops.setattr) {
          throw new FS.ErrnoError(63);
        }
        node.node_ops.setattr(node, {
          mode: (mode & 4095) | (node.mode & ~4095),
          ctime: Date.now()
        });
      },
  lchmod(path, mode) {
        FS.chmod(path, mode, true);
      },
  fchmod(fd, mode) {
        var stream = FS.getStreamChecked(fd);
        FS.chmod(stream.node, mode);
      },
  chown(path, uid, gid, dontFollow) {
        var node;
        if (typeof path == 'string') {
          var lookup = FS.lookupPath(path, { follow: !dontFollow });
          node = lookup.node;
        } else {
          node = path;
        }
        if (!node.node_ops.setattr) {
          throw new FS.ErrnoError(63);
        }
        node.node_ops.setattr(node, {
          timestamp: Date.now()
          // we ignore the uid / gid for now
        });
      },
  lchown(path, uid, gid) {
        FS.chown(path, uid, gid, true);
      },
  fchown(fd, uid, gid) {
        var stream = FS.getStreamChecked(fd);
        FS.chown(stream.node, uid, gid);
      },
  truncate(path, len) {
        if (len < 0) {
          throw new FS.ErrnoError(28);
        }
        var node;
        if (typeof path == 'string') {
          var lookup = FS.lookupPath(path, { follow: true });
          node = lookup.node;
        } else {
          node = path;
        }
        if (!node.node_ops.setattr) {
          throw new FS.ErrnoError(63);
        }
        if (FS.isDir(node.mode)) {
          throw new FS.ErrnoError(31);
        }
        if (!FS.isFile(node.mode)) {
          throw new FS.ErrnoError(28);
        }
        var errCode = FS.nodePermissions(node, 'w');
        if (errCode) {
          throw new FS.ErrnoError(errCode);
        }
        node.node_ops.setattr(node, {
          size: len,
          timestamp: Date.now()
        });
      },
  ftruncate(fd, len) {
        var stream = FS.getStreamChecked(fd);
        if ((stream.flags & 2097155) === 0) {
          throw new FS.ErrnoError(28);
        }
        FS.truncate(stream.node, len);
      },
  utime(path, atime, mtime) {
        var lookup = FS.lookupPath(path, { follow: true });
        var node = lookup.node;
        node.node_ops.setattr(node, {
          atime: atime,
          mtime: mtime
        });
      },
  open(path, flags, mode = 0o666) {
        if (path === "") {
          throw new FS.ErrnoError(44);
        }
        flags = typeof flags == 'string' ? FS_modeStringToFlags(flags) : flags;
        if ((flags & 64)) {
          mode = (mode & 4095) | 32768;
        } else {
          mode = 0;
        }
        var node;
        if (typeof path == 'object') {
          node = path;
        } else {
          // noent_okay makes it so that if the final component of the path
          // doesn't exist, lookupPath returns `node: undefined`. `path` will be
          // updated to point to the target of all symlinks.
          var lookup = FS.lookupPath(path, {
            follow: !(flags & 131072),
            noent_okay: true
          });
          node = lookup.node;
          path = lookup.path;
        }
        // perhaps we need to create the node
        var created = false;
        if ((flags & 64)) {
          if (node) {
            // if O_CREAT and O_EXCL are set, error out if the node already exists
            if ((flags & 128)) {
              throw new FS.ErrnoError(20);
            }
          } else {
            // node doesn't exist, try to create it
            node = FS.mknod(path, mode, 0);
            created = true;
          }
        }
        if (!node) {
          throw new FS.ErrnoError(44);
        }
        // can't truncate a device
        if (FS.isChrdev(node.mode)) {
          flags &= ~512;
        }
        // if asked only for a directory, then this must be one
        if ((flags & 65536) && !FS.isDir(node.mode)) {
          throw new FS.ErrnoError(54);
        }
        // check permissions, if this is not a file we just created now (it is ok to
        // create and write to a file with read-only permissions; it is read-only
        // for later use)
        if (!created) {
          var errCode = FS.mayOpen(node, flags);
          if (errCode) {
            throw new FS.ErrnoError(errCode);
          }
        }
        // do truncation if necessary
        if ((flags & 512) && !created) {
          FS.truncate(node, 0);
        }
        // we've already handled these, don't pass down to the underlying vfs
        flags &= ~(128 | 512 | 131072);
  
        // register the stream with the filesystem
        var stream = FS.createStream({
          node,
          path: FS.getPath(node),  // we want the absolute path to the node
          flags,
          seekable: true,
          position: 0,
          stream_ops: node.stream_ops,
          // used by the file family libc calls (fopen, fwrite, ferror, etc.)
          ungotten: [],
          error: false
        });
        // call the new stream's open function
        if (stream.stream_ops.open) {
          stream.stream_ops.open(stream);
        }
        if (Module['logReadFiles'] && !(flags & 1)) {
          if (!(path in FS.readFiles)) {
            FS.readFiles[path] = 1;
          }
        }
        return stream;
      },
  close(stream) {
        if (FS.isClosed(stream)) {
          throw new FS.ErrnoError(8);
        }
        if (stream.getdents) stream.getdents = null; // free readdir state
        try {
          if (stream.stream_ops.close) {
            stream.stream_ops.close(stream);
          }
        } catch (e) {
          throw e;
        } finally {
          FS.closeStream(stream.fd);
        }
        stream.fd = null;
      },
  isClosed(stream) {
        return stream.fd === null;
      },
  llseek(stream, offset, whence) {
        if (FS.isClosed(stream)) {
          throw new FS.ErrnoError(8);
        }
        if (!stream.seekable || !stream.stream_ops.llseek) {
          throw new FS.ErrnoError(70);
        }
        if (whence != 0 && whence != 1 && whence != 2) {
          throw new FS.ErrnoError(28);
        }
        stream.position = stream.stream_ops.llseek(stream, offset, whence);
        stream.ungotten = [];
        return stream.position;
      },
  read(stream, buffer, offset, length, position) {
        assert(offset >= 0);
        if (length < 0 || position < 0) {
          throw new FS.ErrnoError(28);
        }
        if (FS.isClosed(stream)) {
          throw new FS.ErrnoError(8);
        }
        if ((stream.flags & 2097155) === 1) {
          throw new FS.ErrnoError(8);
        }
        if (FS.isDir(stream.node.mode)) {
          throw new FS.ErrnoError(31);
        }
        if (!stream.stream_ops.read) {
          throw new FS.ErrnoError(28);
        }
        var seeking = typeof position != 'undefined';
        if (!seeking) {
          position = stream.position;
        } else if (!stream.seekable) {
          throw new FS.ErrnoError(70);
        }
        var bytesRead = stream.stream_ops.read(stream, buffer, offset, length, position);
        if (!seeking) stream.position += bytesRead;
        return bytesRead;
      },
  write(stream, buffer, offset, length, position, canOwn) {
        assert(offset >= 0);
        if (length < 0 || position < 0) {
          throw new FS.ErrnoError(28);
        }
        if (FS.isClosed(stream)) {
          throw new FS.ErrnoError(8);
        }
        if ((stream.flags & 2097155) === 0) {
          throw new FS.ErrnoError(8);
        }
        if (FS.isDir(stream.node.mode)) {
          throw new FS.ErrnoError(31);
        }
        if (!stream.stream_ops.write) {
          throw new FS.ErrnoError(28);
        }
        if (stream.seekable && stream.flags & 1024) {
          // seek to the end before writing in append mode
          FS.llseek(stream, 0, 2);
        }
        var seeking = typeof position != 'undefined';
        if (!seeking) {
          position = stream.position;
        } else if (!stream.seekable) {
          throw new FS.ErrnoError(70);
        }
        var bytesWritten = stream.stream_ops.write(stream, buffer, offset, length, position, canOwn);
        if (!seeking) stream.position += bytesWritten;
        return bytesWritten;
      },
  allocate(stream, offset, length) {
        if (FS.isClosed(stream)) {
          throw new FS.ErrnoError(8);
        }
        if (offset < 0 || length <= 0) {
          throw new FS.ErrnoError(28);
        }
        if ((stream.flags & 2097155) === 0) {
          throw new FS.ErrnoError(8);
        }
        if (!FS.isFile(stream.node.mode) && !FS.isDir(stream.node.mode)) {
          throw new FS.ErrnoError(43);
        }
        if (!stream.stream_ops.allocate) {
          throw new FS.ErrnoError(138);
        }
        stream.stream_ops.allocate(stream, offset, length);
      },
  mmap(stream, length, position, prot, flags) {
        // User requests writing to file (prot & PROT_WRITE != 0).
        // Checking if we have permissions to write to the file unless
        // MAP_PRIVATE flag is set. According to POSIX spec it is possible
        // to write to file opened in read-only mode with MAP_PRIVATE flag,
        // as all modifications will be visible only in the memory of
        // the current process.
        if ((prot & 2) !== 0
            && (flags & 2) === 0
            && (stream.flags & 2097155) !== 2) {
          throw new FS.ErrnoError(2);
        }
        if ((stream.flags & 2097155) === 1) {
          throw new FS.ErrnoError(2);
        }
        if (!stream.stream_ops.mmap) {
          throw new FS.ErrnoError(43);
        }
        if (!length) {
          throw new FS.ErrnoError(28);
        }
        return stream.stream_ops.mmap(stream, length, position, prot, flags);
      },
  msync(stream, buffer, offset, length, mmapFlags) {
        assert(offset >= 0);
        if (!stream.stream_ops.msync) {
          return 0;
        }
        return stream.stream_ops.msync(stream, buffer, offset, length, mmapFlags);
      },
  ioctl(stream, cmd, arg) {
        if (!stream.stream_ops.ioctl) {
          throw new FS.ErrnoError(59);
        }
        return stream.stream_ops.ioctl(stream, cmd, arg);
      },
  readFile(path, opts = {}) {
        opts.flags = opts.flags || 0;
        opts.encoding = opts.encoding || 'binary';
        if (opts.encoding !== 'utf8' && opts.encoding !== 'binary') {
          throw new Error(`Invalid encoding type "${opts.encoding}"`);
        }
        var ret;
        var stream = FS.open(path, opts.flags);
        var stat = FS.stat(path);
        var length = stat.size;
        var buf = new Uint8Array(length);
        FS.read(stream, buf, 0, length, 0);
        if (opts.encoding === 'utf8') {
          ret = UTF8ArrayToString(buf);
        } else if (opts.encoding === 'binary') {
          ret = buf;
        }
        FS.close(stream);
        return ret;
      },
  writeFile(path, data, opts = {}) {
        opts.flags = opts.flags || 577;
        var stream = FS.open(path, opts.flags, opts.mode);
        if (typeof data == 'string') {
          var buf = new Uint8Array(lengthBytesUTF8(data)+1);
          var actualNumBytes = stringToUTF8Array(data, buf, 0, buf.length);
          FS.write(stream, buf, 0, actualNumBytes, undefined, opts.canOwn);
        } else if (ArrayBuffer.isView(data)) {
          FS.write(stream, data, 0, data.byteLength, undefined, opts.canOwn);
        } else {
          throw new Error('Unsupported data type');
        }
        FS.close(stream);
      },
  cwd:() => FS.currentPath,
  chdir(path) {
        var lookup = FS.lookupPath(path, { follow: true });
        if (lookup.node === null) {
          throw new FS.ErrnoError(44);
        }
        if (!FS.isDir(lookup.node.mode)) {
          throw new FS.ErrnoError(54);
        }
        var errCode = FS.nodePermissions(lookup.node, 'x');
        if (errCode) {
          throw new FS.ErrnoError(errCode);
        }
        FS.currentPath = lookup.path;
      },
  createDefaultDirectories() {
        FS.mkdir('/tmp');
        FS.mkdir('/home');
        FS.mkdir('/home/web_user');
      },
  createDefaultDevices() {
        // create /dev
        FS.mkdir('/dev');
        // setup /dev/null
        FS.registerDevice(FS.makedev(1, 3), {
          read: () => 0,
          write: (stream, buffer, offset, length, pos) => length,
          llseek: () => 0,
        });
        FS.mkdev('/dev/null', FS.makedev(1, 3));
        // setup /dev/tty and /dev/tty1
        // stderr needs to print output using err() rather than out()
        // so we register a second tty just for it.
        TTY.register(FS.makedev(5, 0), TTY.default_tty_ops);
        TTY.register(FS.makedev(6, 0), TTY.default_tty1_ops);
        FS.mkdev('/dev/tty', FS.makedev(5, 0));
        FS.mkdev('/dev/tty1', FS.makedev(6, 0));
        // setup /dev/[u]random
        // use a buffer to avoid overhead of individual crypto calls per byte
        var randomBuffer = new Uint8Array(1024), randomLeft = 0;
        var randomByte = () => {
          if (randomLeft === 0) {
            randomLeft = randomFill(randomBuffer).byteLength;
          }
          return randomBuffer[--randomLeft];
        };
        FS.createDevice('/dev', 'random', randomByte);
        FS.createDevice('/dev', 'urandom', randomByte);
        // we're not going to emulate the actual shm device,
        // just create the tmp dirs that reside in it commonly
        FS.mkdir('/dev/shm');
        FS.mkdir('/dev/shm/tmp');
      },
  createSpecialDirectories() {
        // create /proc/self/fd which allows /proc/self/fd/6 => readlink gives the
        // name of the stream for fd 6 (see test_unistd_ttyname)
        FS.mkdir('/proc');
        var proc_self = FS.mkdir('/proc/self');
        FS.mkdir('/proc/self/fd');
        FS.mount({
          mount() {
            var node = FS.createNode(proc_self, 'fd', 16895, 73);
            node.stream_ops = {
              llseek: MEMFS.stream_ops.llseek,
            };
            node.node_ops = {
              lookup(parent, name) {
                var fd = +name;
                var stream = FS.getStreamChecked(fd);
                var ret = {
                  parent: null,
                  mount: { mountpoint: 'fake' },
                  node_ops: { readlink: () => stream.path },
                  id: fd + 1,
                };
                ret.parent = ret; // make it look like a simple root node
                return ret;
              },
              readdir() {
                return Array.from(FS.streams.entries())
                  .filter(([k, v]) => v)
                  .map(([k, v]) => k.toString());
              }
            };
            return node;
          }
        }, {}, '/proc/self/fd');
      },
  createStandardStreams(input, output, error) {
        // TODO deprecate the old functionality of a single
        // input / output callback and that utilizes FS.createDevice
        // and instead require a unique set of stream ops
  
        // by default, we symlink the standard streams to the
        // default tty devices. however, if the standard streams
        // have been overwritten we create a unique device for
        // them instead.
        if (input) {
          FS.createDevice('/dev', 'stdin', input);
        } else {
          FS.symlink('/dev/tty', '/dev/stdin');
        }
        if (output) {
          FS.createDevice('/dev', 'stdout', null, output);
        } else {
          FS.symlink('/dev/tty', '/dev/stdout');
        }
        if (error) {
          FS.createDevice('/dev', 'stderr', null, error);
        } else {
          FS.symlink('/dev/tty1', '/dev/stderr');
        }
  
        // open default streams for the stdin, stdout and stderr devices
        var stdin = FS.open('/dev/stdin', 0);
        var stdout = FS.open('/dev/stdout', 1);
        var stderr = FS.open('/dev/stderr', 1);
        assert(stdin.fd === 0, `invalid handle for stdin (${stdin.fd})`);
        assert(stdout.fd === 1, `invalid handle for stdout (${stdout.fd})`);
        assert(stderr.fd === 2, `invalid handle for stderr (${stderr.fd})`);
      },
  staticInit() {
        FS.nameTable = new Array(4096);
  
        FS.mount(MEMFS, {}, '/');
  
        FS.createDefaultDirectories();
        FS.createDefaultDevices();
        FS.createSpecialDirectories();
  
        FS.filesystems = {
          'MEMFS': MEMFS,
        };
      },
  init(input, output, error) {
        assert(!FS.initialized, 'FS.init was previously called. If you want to initialize later with custom parameters, remove any earlier calls (note that one is automatically added to the generated code)');
        FS.initialized = true;
  
        // Allow Module.stdin etc. to provide defaults, if none explicitly passed to us here
        input ??= Module['stdin'];
        output ??= Module['stdout'];
        error ??= Module['stderr'];
  
        FS.createStandardStreams(input, output, error);
      },
  quit() {
        FS.initialized = false;
        // force-flush all streams, so we get musl std streams printed out
        _fflush(0);
        // close all of our streams
        for (var i = 0; i < FS.streams.length; i++) {
          var stream = FS.streams[i];
          if (!stream) {
            continue;
          }
          FS.close(stream);
        }
      },
  findObject(path, dontResolveLastLink) {
        var ret = FS.analyzePath(path, dontResolveLastLink);
        if (!ret.exists) {
          return null;
        }
        return ret.object;
      },
  analyzePath(path, dontResolveLastLink) {
        // operate from within the context of the symlink's target
        try {
          var lookup = FS.lookupPath(path, { follow: !dontResolveLastLink });
          path = lookup.path;
        } catch (e) {
        }
        var ret = {
          isRoot: false, exists: false, error: 0, name: null, path: null, object: null,
          parentExists: false, parentPath: null, parentObject: null
        };
        try {
          var lookup = FS.lookupPath(path, { parent: true });
          ret.parentExists = true;
          ret.parentPath = lookup.path;
          ret.parentObject = lookup.node;
          ret.name = PATH.basename(path);
          lookup = FS.lookupPath(path, { follow: !dontResolveLastLink });
          ret.exists = true;
          ret.path = lookup.path;
          ret.object = lookup.node;
          ret.name = lookup.node.name;
          ret.isRoot = lookup.path === '/';
        } catch (e) {
          ret.error = e.errno;
        };
        return ret;
      },
  createPath(parent, path, canRead, canWrite) {
        parent = typeof parent == 'string' ? parent : FS.getPath(parent);
        var parts = path.split('/').reverse();
        while (parts.length) {
          var part = parts.pop();
          if (!part) continue;
          var current = PATH.join2(parent, part);
          try {
            FS.mkdir(current);
          } catch (e) {
            // ignore EEXIST
          }
          parent = current;
        }
        return current;
      },
  createFile(parent, name, properties, canRead, canWrite) {
        var path = PATH.join2(typeof parent == 'string' ? parent : FS.getPath(parent), name);
        var mode = FS_getMode(canRead, canWrite);
        return FS.create(path, mode);
      },
  createDataFile(parent, name, data, canRead, canWrite, canOwn) {
        var path = name;
        if (parent) {
          parent = typeof parent == 'string' ? parent : FS.getPath(parent);
          path = name ? PATH.join2(parent, name) : parent;
        }
        var mode = FS_getMode(canRead, canWrite);
        var node = FS.create(path, mode);
        if (data) {
          if (typeof data == 'string') {
            var arr = new Array(data.length);
            for (var i = 0, len = data.length; i < len; ++i) arr[i] = data.charCodeAt(i);
            data = arr;
          }
          // make sure we can write to the file
          FS.chmod(node, mode | 146);
          var stream = FS.open(node, 577);
          FS.write(stream, data, 0, data.length, 0, canOwn);
          FS.close(stream);
          FS.chmod(node, mode);
        }
      },
  createDevice(parent, name, input, output) {
        var path = PATH.join2(typeof parent == 'string' ? parent : FS.getPath(parent), name);
        var mode = FS_getMode(!!input, !!output);
        FS.createDevice.major ??= 64;
        var dev = FS.makedev(FS.createDevice.major++, 0);
        // Create a fake device that a set of stream ops to emulate
        // the old behavior.
        FS.registerDevice(dev, {
          open(stream) {
            stream.seekable = false;
          },
          close(stream) {
            // flush any pending line data
            if (output?.buffer?.length) {
              output(10);
            }
          },
          read(stream, buffer, offset, length, pos /* ignored */) {
            var bytesRead = 0;
            for (var i = 0; i < length; i++) {
              var result;
              try {
                result = input();
              } catch (e) {
                throw new FS.ErrnoError(29);
              }
              if (result === undefined && bytesRead === 0) {
                throw new FS.ErrnoError(6);
              }
              if (result === null || result === undefined) break;
              bytesRead++;
              buffer[offset+i] = result;
            }
            if (bytesRead) {
              stream.node.atime = Date.now();
            }
            return bytesRead;
          },
          write(stream, buffer, offset, length, pos) {
            for (var i = 0; i < length; i++) {
              try {
                output(buffer[offset+i]);
              } catch (e) {
                throw new FS.ErrnoError(29);
              }
            }
            if (length) {
              stream.node.mtime = stream.node.ctime = Date.now();
            }
            return i;
          }
        });
        return FS.mkdev(path, mode, dev);
      },
  forceLoadFile(obj) {
        if (obj.isDevice || obj.isFolder || obj.link || obj.contents) return true;
        if (typeof XMLHttpRequest != 'undefined') {
          throw new Error("Lazy loading should have been performed (contents set) in createLazyFile, but it was not. Lazy loading only works in web workers. Use --embed-file or --preload-file in emcc on the main thread.");
        } else { // Command-line.
          try {
            obj.contents = readBinary(obj.url);
            obj.usedBytes = obj.contents.length;
          } catch (e) {
            throw new FS.ErrnoError(29);
          }
        }
      },
  createLazyFile(parent, name, url, canRead, canWrite) {
        // Lazy chunked Uint8Array (implements get and length from Uint8Array).
        // Actual getting is abstracted away for eventual reuse.
        class LazyUint8Array {
          lengthKnown = false;
          chunks = []; // Loaded chunks. Index is the chunk number
          get(idx) {
            if (idx > this.length-1 || idx < 0) {
              return undefined;
            }
            var chunkOffset = idx % this.chunkSize;
            var chunkNum = (idx / this.chunkSize)|0;
            return this.getter(chunkNum)[chunkOffset];
          }
          setDataGetter(getter) {
            this.getter = getter;
          }
          cacheLength() {
            // Find length
            var xhr = new XMLHttpRequest();
            xhr.open('HEAD', url, false);
            xhr.send(null);
            if (!(xhr.status >= 200 && xhr.status < 300 || xhr.status === 304)) throw new Error("Couldn't load " + url + ". Status: " + xhr.status);
            var datalength = Number(xhr.getResponseHeader("Content-length"));
            var header;
            var hasByteServing = (header = xhr.getResponseHeader("Accept-Ranges")) && header === "bytes";
            var usesGzip = (header = xhr.getResponseHeader("Content-Encoding")) && header === "gzip";
  
            var chunkSize = 1024*1024; // Chunk size in bytes
  
            if (!hasByteServing) chunkSize = datalength;
  
            // Function to get a range from the remote URL.
            var doXHR = (from, to) => {
              if (from > to) throw new Error("invalid range (" + from + ", " + to + ") or no bytes requested!");
              if (to > datalength-1) throw new Error("only " + datalength + " bytes available! programmer error!");
  
              // TODO: Use mozResponseArrayBuffer, responseStream, etc. if available.
              var xhr = new XMLHttpRequest();
              xhr.open('GET', url, false);
              if (datalength !== chunkSize) xhr.setRequestHeader("Range", "bytes=" + from + "-" + to);
  
              // Some hints to the browser that we want binary data.
              xhr.responseType = 'arraybuffer';
              if (xhr.overrideMimeType) {
                xhr.overrideMimeType('text/plain; charset=x-user-defined');
              }
  
              xhr.send(null);
              if (!(xhr.status >= 200 && xhr.status < 300 || xhr.status === 304)) throw new Error("Couldn't load " + url + ". Status: " + xhr.status);
              if (xhr.response !== undefined) {
                return new Uint8Array(/** @type{Array<number>} */(xhr.response || []));
              }
              return intArrayFromString(xhr.responseText || '', true);
            };
            var lazyArray = this;
            lazyArray.setDataGetter((chunkNum) => {
              var start = chunkNum * chunkSize;
              var end = (chunkNum+1) * chunkSize - 1; // including this byte
              end = Math.min(end, datalength-1); // if datalength-1 is selected, this is the last block
              if (typeof lazyArray.chunks[chunkNum] == 'undefined') {
                lazyArray.chunks[chunkNum] = doXHR(start, end);
              }
              if (typeof lazyArray.chunks[chunkNum] == 'undefined') throw new Error('doXHR failed!');
              return lazyArray.chunks[chunkNum];
            });
  
            if (usesGzip || !datalength) {
              // if the server uses gzip or doesn't supply the length, we have to download the whole file to get the (uncompressed) length
              chunkSize = datalength = 1; // this will force getter(0)/doXHR do download the whole file
              datalength = this.getter(0).length;
              chunkSize = datalength;
              out("LazyFiles on gzip forces download of the whole file when length is accessed");
            }
  
            this._length = datalength;
            this._chunkSize = chunkSize;
            this.lengthKnown = true;
          }
          get length() {
            if (!this.lengthKnown) {
              this.cacheLength();
            }
            return this._length;
          }
          get chunkSize() {
            if (!this.lengthKnown) {
              this.cacheLength();
            }
            return this._chunkSize;
          }
        }
  
        if (typeof XMLHttpRequest != 'undefined') {
          if (!ENVIRONMENT_IS_WORKER) throw 'Cannot do synchronous binary XHRs outside webworkers in modern browsers. Use --embed-file or --preload-file in emcc';
          var lazyArray = new LazyUint8Array();
          var properties = { isDevice: false, contents: lazyArray };
        } else {
          var properties = { isDevice: false, url: url };
        }
  
        var node = FS.createFile(parent, name, properties, canRead, canWrite);
        // This is a total hack, but I want to get this lazy file code out of the
        // core of MEMFS. If we want to keep this lazy file concept I feel it should
        // be its own thin LAZYFS proxying calls to MEMFS.
        if (properties.contents) {
          node.contents = properties.contents;
        } else if (properties.url) {
          node.contents = null;
          node.url = properties.url;
        }
        // Add a function that defers querying the file size until it is asked the first time.
        Object.defineProperties(node, {
          usedBytes: {
            get: function() { return this.contents.length; }
          }
        });
        // override each stream op with one that tries to force load the lazy file first
        var stream_ops = {};
        var keys = Object.keys(node.stream_ops);
        keys.forEach((key) => {
          var fn = node.stream_ops[key];
          stream_ops[key] = (...args) => {
            FS.forceLoadFile(node);
            return fn(...args);
          };
        });
        function writeChunks(stream, buffer, offset, length, position) {
          var contents = stream.node.contents;
          if (position >= contents.length)
            return 0;
          var size = Math.min(contents.length - position, length);
          assert(size >= 0);
          if (contents.slice) { // normal array
            for (var i = 0; i < size; i++) {
              buffer[offset + i] = contents[position + i];
            }
          } else {
            for (var i = 0; i < size; i++) { // LazyUint8Array from sync binary XHR
              buffer[offset + i] = contents.get(position + i);
            }
          }
          return size;
        }
        // use a custom read function
        stream_ops.read = (stream, buffer, offset, length, position) => {
          FS.forceLoadFile(node);
          return writeChunks(stream, buffer, offset, length, position)
        };
        // use a custom mmap function
        stream_ops.mmap = (stream, length, position, prot, flags) => {
          FS.forceLoadFile(node);
          var ptr = mmapAlloc(length);
          if (!ptr) {
            throw new FS.ErrnoError(48);
          }
          writeChunks(stream, HEAP8, ptr, length, position);
          return { ptr, allocated: true };
        };
        node.stream_ops = stream_ops;
        return node;
      },
  absolutePath() {
        abort('FS.absolutePath has been removed; use PATH_FS.resolve instead');
      },
  createFolder() {
        abort('FS.createFolder has been removed; use FS.mkdir instead');
      },
  createLink() {
        abort('FS.createLink has been removed; use FS.symlink instead');
      },
  joinPath() {
        abort('FS.joinPath has been removed; use PATH.join instead');
      },
  mmapAlloc() {
        abort('FS.mmapAlloc has been replaced by the top level function mmapAlloc');
      },
  standardizePath() {
        abort('FS.standardizePath has been removed; use PATH.normalize instead');
      },
  };
  
  var SYSCALLS = {
  DEFAULT_POLLMASK:5,
  calculateAt(dirfd, path, allowEmpty) {
        if (PATH.isAbs(path)) {
          return path;
        }
        // relative path
        var dir;
        if (dirfd === -100) {
          dir = FS.cwd();
        } else {
          var dirstream = SYSCALLS.getStreamFromFD(dirfd);
          dir = dirstream.path;
        }
        if (path.length == 0) {
          if (!allowEmpty) {
            throw new FS.ErrnoError(44);;
          }
          return dir;
        }
        return dir + '/' + path;
      },
  doStat(func, path, buf) {
        var stat = func(path);
        HEAP32[((buf)>>2)] = stat.dev;checkInt32(stat.dev);
        HEAP32[(((buf)+(4))>>2)] = stat.mode;checkInt32(stat.mode);
        HEAPU32[(((buf)+(8))>>2)] = stat.nlink;checkInt32(stat.nlink);
        HEAP32[(((buf)+(12))>>2)] = stat.uid;checkInt32(stat.uid);
        HEAP32[(((buf)+(16))>>2)] = stat.gid;checkInt32(stat.gid);
        HEAP32[(((buf)+(20))>>2)] = stat.rdev;checkInt32(stat.rdev);
        HEAP64[(((buf)+(24))>>3)] = BigInt(stat.size);checkInt64(stat.size);
        HEAP32[(((buf)+(32))>>2)] = 4096;checkInt32(4096);
        HEAP32[(((buf)+(36))>>2)] = stat.blocks;checkInt32(stat.blocks);
        var atime = stat.atime.getTime();
        var mtime = stat.mtime.getTime();
        var ctime = stat.ctime.getTime();
        HEAP64[(((buf)+(40))>>3)] = BigInt(Math.floor(atime / 1000));checkInt64(Math.floor(atime / 1000));
        HEAPU32[(((buf)+(48))>>2)] = (atime % 1000) * 1000 * 1000;checkInt32((atime % 1000) * 1000 * 1000);
        HEAP64[(((buf)+(56))>>3)] = BigInt(Math.floor(mtime / 1000));checkInt64(Math.floor(mtime / 1000));
        HEAPU32[(((buf)+(64))>>2)] = (mtime % 1000) * 1000 * 1000;checkInt32((mtime % 1000) * 1000 * 1000);
        HEAP64[(((buf)+(72))>>3)] = BigInt(Math.floor(ctime / 1000));checkInt64(Math.floor(ctime / 1000));
        HEAPU32[(((buf)+(80))>>2)] = (ctime % 1000) * 1000 * 1000;checkInt32((ctime % 1000) * 1000 * 1000);
        HEAP64[(((buf)+(88))>>3)] = BigInt(stat.ino);checkInt64(stat.ino);
        return 0;
      },
  doMsync(addr, stream, len, flags, offset) {
        if (!FS.isFile(stream.node.mode)) {
          throw new FS.ErrnoError(43);
        }
        if (flags & 2) {
          // MAP_PRIVATE calls need not to be synced back to underlying fs
          return 0;
        }
        var buffer = HEAPU8.slice(addr, addr + len);
        FS.msync(stream, buffer, offset, len, flags);
      },
  getStreamFromFD(fd) {
        var stream = FS.getStreamChecked(fd);
        return stream;
      },
  varargs:undefined,
  getStr(ptr) {
        var ret = UTF8ToString(ptr);
        return ret;
      },
  };
  function ___syscall_chmod(path, mode) {
  try {
  
      path = SYSCALLS.getStr(path);
      FS.chmod(path, mode);
      return 0;
    } catch (e) {
    if (typeof FS == 'undefined' || !(e.name === 'ErrnoError')) throw e;
    return -e.errno;
  }
  }

  function ___syscall_faccessat(dirfd, path, amode, flags) {
  try {
  
      path = SYSCALLS.getStr(path);
      assert(flags === 0 || flags == 512);
      path = SYSCALLS.calculateAt(dirfd, path);
      if (amode & ~7) {
        // need a valid mode
        return -28;
      }
      var lookup = FS.lookupPath(path, { follow: true });
      var node = lookup.node;
      if (!node) {
        return -44;
      }
      var perms = '';
      if (amode & 4) perms += 'r';
      if (amode & 2) perms += 'w';
      if (amode & 1) perms += 'x';
      if (perms /* otherwise, they've just passed F_OK */ && FS.nodePermissions(node, perms)) {
        return -2;
      }
      return 0;
    } catch (e) {
    if (typeof FS == 'undefined' || !(e.name === 'ErrnoError')) throw e;
    return -e.errno;
  }
  }

  function ___syscall_fchmod(fd, mode) {
  try {
  
      FS.fchmod(fd, mode);
      return 0;
    } catch (e) {
    if (typeof FS == 'undefined' || !(e.name === 'ErrnoError')) throw e;
    return -e.errno;
  }
  }

  function ___syscall_fchown32(fd, owner, group) {
  try {
  
      FS.fchown(fd, owner, group);
      return 0;
    } catch (e) {
    if (typeof FS == 'undefined' || !(e.name === 'ErrnoError')) throw e;
    return -e.errno;
  }
  }

  /** @suppress {duplicate } */
  var syscallGetVarargI = () => {
      assert(SYSCALLS.varargs != undefined);
      // the `+` prepended here is necessary to convince the JSCompiler that varargs is indeed a number.
      var ret = HEAP32[((+SYSCALLS.varargs)>>2)];
      SYSCALLS.varargs += 4;
      return ret;
    };
  var syscallGetVarargP = syscallGetVarargI;
  
  
  function ___syscall_fcntl64(fd, cmd, varargs) {
  SYSCALLS.varargs = varargs;
  try {
  
      var stream = SYSCALLS.getStreamFromFD(fd);
      switch (cmd) {
        case 0: {
          var arg = syscallGetVarargI();
          if (arg < 0) {
            return -28;
          }
          while (FS.streams[arg]) {
            arg++;
          }
          var newStream;
          newStream = FS.dupStream(stream, arg);
          return newStream.fd;
        }
        case 1:
        case 2:
          return 0;  // FD_CLOEXEC makes no sense for a single process.
        case 3:
          return stream.flags;
        case 4: {
          var arg = syscallGetVarargI();
          stream.flags |= arg;
          return 0;
        }
        case 12: {
          var arg = syscallGetVarargP();
          var offset = 0;
          // We're always unlocked.
          HEAP16[(((arg)+(offset))>>1)] = 2;checkInt16(2);
          return 0;
        }
        case 13:
        case 14:
          return 0; // Pretend that the locking is successful.
      }
      return -28;
    } catch (e) {
    if (typeof FS == 'undefined' || !(e.name === 'ErrnoError')) throw e;
    return -e.errno;
  }
  }

  function ___syscall_fstat64(fd, buf) {
  try {
  
      var stream = SYSCALLS.getStreamFromFD(fd);
      return SYSCALLS.doStat(FS.stat, stream.path, buf);
    } catch (e) {
    if (typeof FS == 'undefined' || !(e.name === 'ErrnoError')) throw e;
    return -e.errno;
  }
  }

  var INT53_MAX = 9007199254740992;
  
  var INT53_MIN = -9007199254740992;
  var bigintToI53Checked = (num) => (num < INT53_MIN || num > INT53_MAX) ? NaN : Number(num);
  function ___syscall_ftruncate64(fd, length) {
    length = bigintToI53Checked(length);
  
    
  try {
  
      if (isNaN(length)) return 61;
      FS.ftruncate(fd, length);
      return 0;
    } catch (e) {
    if (typeof FS == 'undefined' || !(e.name === 'ErrnoError')) throw e;
    return -e.errno;
  }
  ;
  }

  
  var stringToUTF8 = (str, outPtr, maxBytesToWrite) => {
      assert(typeof maxBytesToWrite == 'number', 'stringToUTF8(str, outPtr, maxBytesToWrite) is missing the third parameter that specifies the length of the output buffer!');
      return stringToUTF8Array(str, HEAPU8, outPtr, maxBytesToWrite);
    };
  function ___syscall_getcwd(buf, size) {
  try {
  
      if (size === 0) return -28;
      var cwd = FS.cwd();
      var cwdLengthInBytes = lengthBytesUTF8(cwd) + 1;
      if (size < cwdLengthInBytes) return -68;
      stringToUTF8(cwd, buf, size);
      return cwdLengthInBytes;
    } catch (e) {
    if (typeof FS == 'undefined' || !(e.name === 'ErrnoError')) throw e;
    return -e.errno;
  }
  }

  
  function ___syscall_ioctl(fd, op, varargs) {
  SYSCALLS.varargs = varargs;
  try {
  
      var stream = SYSCALLS.getStreamFromFD(fd);
      switch (op) {
        case 21509: {
          if (!stream.tty) return -59;
          return 0;
        }
        case 21505: {
          if (!stream.tty) return -59;
          if (stream.tty.ops.ioctl_tcgets) {
            var termios = stream.tty.ops.ioctl_tcgets(stream);
            var argp = syscallGetVarargP();
            HEAP32[((argp)>>2)] = termios.c_iflag || 0;checkInt32(termios.c_iflag || 0);
            HEAP32[(((argp)+(4))>>2)] = termios.c_oflag || 0;checkInt32(termios.c_oflag || 0);
            HEAP32[(((argp)+(8))>>2)] = termios.c_cflag || 0;checkInt32(termios.c_cflag || 0);
            HEAP32[(((argp)+(12))>>2)] = termios.c_lflag || 0;checkInt32(termios.c_lflag || 0);
            for (var i = 0; i < 32; i++) {
              HEAP8[(argp + i)+(17)] = termios.c_cc[i] || 0;checkInt8(termios.c_cc[i] || 0);
            }
            return 0;
          }
          return 0;
        }
        case 21510:
        case 21511:
        case 21512: {
          if (!stream.tty) return -59;
          return 0; // no-op, not actually adjusting terminal settings
        }
        case 21506:
        case 21507:
        case 21508: {
          if (!stream.tty) return -59;
          if (stream.tty.ops.ioctl_tcsets) {
            var argp = syscallGetVarargP();
            var c_iflag = HEAP32[((argp)>>2)];
            var c_oflag = HEAP32[(((argp)+(4))>>2)];
            var c_cflag = HEAP32[(((argp)+(8))>>2)];
            var c_lflag = HEAP32[(((argp)+(12))>>2)];
            var c_cc = []
            for (var i = 0; i < 32; i++) {
              c_cc.push(HEAP8[(argp + i)+(17)]);
            }
            return stream.tty.ops.ioctl_tcsets(stream.tty, op, { c_iflag, c_oflag, c_cflag, c_lflag, c_cc });
          }
          return 0; // no-op, not actually adjusting terminal settings
        }
        case 21519: {
          if (!stream.tty) return -59;
          var argp = syscallGetVarargP();
          HEAP32[((argp)>>2)] = 0;checkInt32(0);
          return 0;
        }
        case 21520: {
          if (!stream.tty) return -59;
          return -28; // not supported
        }
        case 21531: {
          var argp = syscallGetVarargP();
          return FS.ioctl(stream, op, argp);
        }
        case 21523: {
          // TODO: in theory we should write to the winsize struct that gets
          // passed in, but for now musl doesn't read anything on it
          if (!stream.tty) return -59;
          if (stream.tty.ops.ioctl_tiocgwinsz) {
            var winsize = stream.tty.ops.ioctl_tiocgwinsz(stream.tty);
            var argp = syscallGetVarargP();
            HEAP16[((argp)>>1)] = winsize[0];checkInt16(winsize[0]);
            HEAP16[(((argp)+(2))>>1)] = winsize[1];checkInt16(winsize[1]);
          }
          return 0;
        }
        case 21524: {
          // TODO: technically, this ioctl call should change the window size.
          // but, since emscripten doesn't have any concept of a terminal window
          // yet, we'll just silently throw it away as we do TIOCGWINSZ
          if (!stream.tty) return -59;
          return 0;
        }
        case 21515: {
          if (!stream.tty) return -59;
          return 0;
        }
        default: return -28; // not supported
      }
    } catch (e) {
    if (typeof FS == 'undefined' || !(e.name === 'ErrnoError')) throw e;
    return -e.errno;
  }
  }

  function ___syscall_lstat64(path, buf) {
  try {
  
      path = SYSCALLS.getStr(path);
      return SYSCALLS.doStat(FS.lstat, path, buf);
    } catch (e) {
    if (typeof FS == 'undefined' || !(e.name === 'ErrnoError')) throw e;
    return -e.errno;
  }
  }

  function ___syscall_mkdirat(dirfd, path, mode) {
  try {
  
      path = SYSCALLS.getStr(path);
      path = SYSCALLS.calculateAt(dirfd, path);
      FS.mkdir(path, mode, 0);
      return 0;
    } catch (e) {
    if (typeof FS == 'undefined' || !(e.name === 'ErrnoError')) throw e;
    return -e.errno;
  }
  }

  function ___syscall_newfstatat(dirfd, path, buf, flags) {
  try {
  
      path = SYSCALLS.getStr(path);
      var nofollow = flags & 256;
      var allowEmpty = flags & 4096;
      flags = flags & (~6400);
      assert(!flags, `unknown flags in __syscall_newfstatat: ${flags}`);
      path = SYSCALLS.calculateAt(dirfd, path, allowEmpty);
      return SYSCALLS.doStat(nofollow ? FS.lstat : FS.stat, path, buf);
    } catch (e) {
    if (typeof FS == 'undefined' || !(e.name === 'ErrnoError')) throw e;
    return -e.errno;
  }
  }

  
  function ___syscall_openat(dirfd, path, flags, varargs) {
  SYSCALLS.varargs = varargs;
  try {
  
      path = SYSCALLS.getStr(path);
      path = SYSCALLS.calculateAt(dirfd, path);
      var mode = varargs ? syscallGetVarargI() : 0;
      return FS.open(path, flags, mode).fd;
    } catch (e) {
    if (typeof FS == 'undefined' || !(e.name === 'ErrnoError')) throw e;
    return -e.errno;
  }
  }

  
  
  function ___syscall_readlinkat(dirfd, path, buf, bufsize) {
  try {
  
      path = SYSCALLS.getStr(path);
      path = SYSCALLS.calculateAt(dirfd, path);
      if (bufsize <= 0) return -28;
      var ret = FS.readlink(path);
  
      var len = Math.min(bufsize, lengthBytesUTF8(ret));
      var endChar = HEAP8[buf+len];
      stringToUTF8(ret, buf, bufsize+1);
      // readlink is one of the rare functions that write out a C string, but does never append a null to the output buffer(!)
      // stringToUTF8() always appends a null byte, so restore the character under the null byte after the write.
      HEAP8[buf+len] = endChar;
      return len;
    } catch (e) {
    if (typeof FS == 'undefined' || !(e.name === 'ErrnoError')) throw e;
    return -e.errno;
  }
  }

  function ___syscall_rmdir(path) {
  try {
  
      path = SYSCALLS.getStr(path);
      FS.rmdir(path);
      return 0;
    } catch (e) {
    if (typeof FS == 'undefined' || !(e.name === 'ErrnoError')) throw e;
    return -e.errno;
  }
  }

  function ___syscall_stat64(path, buf) {
  try {
  
      path = SYSCALLS.getStr(path);
      return SYSCALLS.doStat(FS.stat, path, buf);
    } catch (e) {
    if (typeof FS == 'undefined' || !(e.name === 'ErrnoError')) throw e;
    return -e.errno;
  }
  }

  function ___syscall_unlinkat(dirfd, path, flags) {
  try {
  
      path = SYSCALLS.getStr(path);
      path = SYSCALLS.calculateAt(dirfd, path);
      if (flags === 0) {
        FS.unlink(path);
      } else if (flags === 512) {
        FS.rmdir(path);
      } else {
        abort('Invalid flags passed to unlinkat');
      }
      return 0;
    } catch (e) {
    if (typeof FS == 'undefined' || !(e.name === 'ErrnoError')) throw e;
    return -e.errno;
  }
  }

  var readI53FromI64 = (ptr) => {
      return HEAPU32[((ptr)>>2)] + HEAP32[(((ptr)+(4))>>2)] * 4294967296;
    };
  
  function ___syscall_utimensat(dirfd, path, times, flags) {
  try {
  
      path = SYSCALLS.getStr(path);
      assert(flags === 0);
      path = SYSCALLS.calculateAt(dirfd, path, true);
      var now = Date.now(), atime, mtime;
      if (!times) {
        atime = now;
        mtime = now;
      } else {
        var seconds = readI53FromI64(times);
        var nanoseconds = HEAP32[(((times)+(8))>>2)];
        if (nanoseconds == 1073741823) {
          atime = now;
        } else if (nanoseconds == 1073741822) {
          atime = null;
        } else {
          atime = (seconds*1000) + (nanoseconds/(1000*1000));
        }
        times += 16;
        seconds = readI53FromI64(times);
        nanoseconds = HEAP32[(((times)+(8))>>2)];
        if (nanoseconds == 1073741823) {
          mtime = now;
        } else if (nanoseconds == 1073741822) {
          mtime = null;
        } else {
          mtime = (seconds*1000) + (nanoseconds/(1000*1000));
        }
      }
      // null here means UTIME_OMIT was passed. If both were set to UTIME_OMIT then
      // we can skip the call completely.
      if ((mtime ?? atime) !== null) {
        FS.utime(path, atime, mtime);
      }
      return 0;
    } catch (e) {
    if (typeof FS == 'undefined' || !(e.name === 'ErrnoError')) throw e;
    return -e.errno;
  }
  }

  var __abort_js = () =>
      abort('native code called abort()');

  var isLeapYear = (year) => year%4 === 0 && (year%100 !== 0 || year%400 === 0);
  
  var MONTH_DAYS_LEAP_CUMULATIVE = [0,31,60,91,121,152,182,213,244,274,305,335];
  
  var MONTH_DAYS_REGULAR_CUMULATIVE = [0,31,59,90,120,151,181,212,243,273,304,334];
  var ydayFromDate = (date) => {
      var leap = isLeapYear(date.getFullYear());
      var monthDaysCumulative = (leap ? MONTH_DAYS_LEAP_CUMULATIVE : MONTH_DAYS_REGULAR_CUMULATIVE);
      var yday = monthDaysCumulative[date.getMonth()] + date.getDate() - 1; // -1 since it's days since Jan 1
  
      return yday;
    };
  
  function __localtime_js(time, tmPtr) {
    time = bigintToI53Checked(time);
  
    
      var date = new Date(time*1000);
      HEAP32[((tmPtr)>>2)] = date.getSeconds();checkInt32(date.getSeconds());
      HEAP32[(((tmPtr)+(4))>>2)] = date.getMinutes();checkInt32(date.getMinutes());
      HEAP32[(((tmPtr)+(8))>>2)] = date.getHours();checkInt32(date.getHours());
      HEAP32[(((tmPtr)+(12))>>2)] = date.getDate();checkInt32(date.getDate());
      HEAP32[(((tmPtr)+(16))>>2)] = date.getMonth();checkInt32(date.getMonth());
      HEAP32[(((tmPtr)+(20))>>2)] = date.getFullYear()-1900;checkInt32(date.getFullYear()-1900);
      HEAP32[(((tmPtr)+(24))>>2)] = date.getDay();checkInt32(date.getDay());
  
      var yday = ydayFromDate(date)|0;
      HEAP32[(((tmPtr)+(28))>>2)] = yday;checkInt32(yday);
      HEAP32[(((tmPtr)+(36))>>2)] = -(date.getTimezoneOffset() * 60);checkInt32(-(date.getTimezoneOffset() * 60));
  
      // Attention: DST is in December in South, and some regions don't have DST at all.
      var start = new Date(date.getFullYear(), 0, 1);
      var summerOffset = new Date(date.getFullYear(), 6, 1).getTimezoneOffset();
      var winterOffset = start.getTimezoneOffset();
      var dst = (summerOffset != winterOffset && date.getTimezoneOffset() == Math.min(winterOffset, summerOffset))|0;
      HEAP32[(((tmPtr)+(32))>>2)] = dst;checkInt32(dst);
    ;
  }

  
  
  
  
  
  function __mmap_js(len, prot, flags, fd, offset, allocated, addr) {
    offset = bigintToI53Checked(offset);
  
    
  try {
  
      if (isNaN(offset)) return 61;
      var stream = SYSCALLS.getStreamFromFD(fd);
      var res = FS.mmap(stream, len, offset, prot, flags);
      var ptr = res.ptr;
      HEAP32[((allocated)>>2)] = res.allocated;checkInt32(res.allocated);
      HEAPU32[((addr)>>2)] = ptr;
      return 0;
    } catch (e) {
    if (typeof FS == 'undefined' || !(e.name === 'ErrnoError')) throw e;
    return -e.errno;
  }
  ;
  }

  
  function __munmap_js(addr, len, prot, flags, fd, offset) {
    offset = bigintToI53Checked(offset);
  
    
  try {
  
      var stream = SYSCALLS.getStreamFromFD(fd);
      if (prot & 2) {
        SYSCALLS.doMsync(addr, stream, len, flags, offset);
      }
    } catch (e) {
    if (typeof FS == 'undefined' || !(e.name === 'ErrnoError')) throw e;
    return -e.errno;
  }
  ;
  }

  
  var __tzset_js = (timezone, daylight, std_name, dst_name) => {
      // TODO: Use (malleable) environment variables instead of system settings.
      var currentYear = new Date().getFullYear();
      var winter = new Date(currentYear, 0, 1);
      var summer = new Date(currentYear, 6, 1);
      var winterOffset = winter.getTimezoneOffset();
      var summerOffset = summer.getTimezoneOffset();
  
      // Local standard timezone offset. Local standard time is not adjusted for
      // daylight savings.  This code uses the fact that getTimezoneOffset returns
      // a greater value during Standard Time versus Daylight Saving Time (DST).
      // Thus it determines the expected output during Standard Time, and it
      // compares whether the output of the given date the same (Standard) or less
      // (DST).
      var stdTimezoneOffset = Math.max(winterOffset, summerOffset);
  
      // timezone is specified as seconds west of UTC ("The external variable
      // `timezone` shall be set to the difference, in seconds, between
      // Coordinated Universal Time (UTC) and local standard time."), the same
      // as returned by stdTimezoneOffset.
      // See http://pubs.opengroup.org/onlinepubs/009695399/functions/tzset.html
      HEAPU32[((timezone)>>2)] = stdTimezoneOffset * 60;checkInt32(stdTimezoneOffset * 60);
  
      HEAP32[((daylight)>>2)] = Number(winterOffset != summerOffset);checkInt32(Number(winterOffset != summerOffset));
  
      var extractZone = (timezoneOffset) => {
        // Why inverse sign?
        // Read here https://developer.mozilla.org/en-US/docs/Web/JavaScript/Reference/Global_Objects/Date/getTimezoneOffset
        var sign = timezoneOffset >= 0 ? "-" : "+";
  
        var absOffset = Math.abs(timezoneOffset)
        var hours = String(Math.floor(absOffset / 60)).padStart(2, "0");
        var minutes = String(absOffset % 60).padStart(2, "0");
  
        return `UTC${sign}${hours}${minutes}`;
      }
  
      var winterName = extractZone(winterOffset);
      var summerName = extractZone(summerOffset);
      assert(winterName);
      assert(summerName);
      assert(lengthBytesUTF8(winterName) <= 16, `timezone name truncated to fit in TZNAME_MAX (${winterName})`);
      assert(lengthBytesUTF8(summerName) <= 16, `timezone name truncated to fit in TZNAME_MAX (${summerName})`);
      if (summerOffset < winterOffset) {
        // Northern hemisphere
        stringToUTF8(winterName, std_name, 17);
        stringToUTF8(summerName, dst_name, 17);
      } else {
        stringToUTF8(winterName, dst_name, 17);
        stringToUTF8(summerName, std_name, 17);
      }
    };

  var _emscripten_get_now = () => performance.now();
  
  var _emscripten_date_now = () => Date.now();
  
  var nowIsMonotonic = 1;
  
  var checkWasiClock = (clock_id) => clock_id >= 0 && clock_id <= 3;
  
  function _clock_time_get(clk_id, ignored_precision, ptime) {
    ignored_precision = bigintToI53Checked(ignored_precision);
  
    
      if (!checkWasiClock(clk_id)) {
        return 28;
      }
      var now;
      // all wasi clocks but realtime are monotonic
      if (clk_id === 0) {
        now = _emscripten_date_now();
      } else if (nowIsMonotonic) {
        now = _emscripten_get_now();
      } else {
        return 52;
      }
      // "now" is in ms, and wasi times are in ns.
      var nsec = Math.round(now * 1000 * 1000);
      HEAP64[((ptime)>>3)] = BigInt(nsec);checkInt64(nsec);
      return 0;
    ;
  }


  var getHeapMax = () =>
      // Stay one Wasm page short of 4GB: while e.g. Chrome is able to allocate
      // full 4GB Wasm memories, the size will wrap back to 0 bytes in Wasm side
      // for any code that deals with heap sizes, which would require special
      // casing all heap size related code to treat 0 specially.
      2147483648;
  var _emscripten_get_heap_max = () => getHeapMax();


  
  
  
  var growMemory = (size) => {
      var b = wasmMemory.buffer;
      var pages = ((size - b.byteLength + 65535) / 65536) | 0;
      try {
        // round size grow request up to wasm page size (fixed 64KB per spec)
        wasmMemory.grow(pages); // .grow() takes a delta compared to the previous size
        updateMemoryViews();
        return 1 /*success*/;
      } catch(e) {
        err(`growMemory: Attempted to grow heap from ${b.byteLength} bytes to ${size} bytes, but got error: ${e}`);
      }
      // implicit 0 return to save code size (caller will cast "undefined" into 0
      // anyhow)
    };
  var _emscripten_resize_heap = (requestedSize) => {
      var oldSize = HEAPU8.length;
      // With CAN_ADDRESS_2GB or MEMORY64, pointers are already unsigned.
      requestedSize >>>= 0;
      // With multithreaded builds, races can happen (another thread might increase the size
      // in between), so return a failure, and let the caller retry.
      assert(requestedSize > oldSize);
  
      // Memory resize rules:
      // 1.  Always increase heap size to at least the requested size, rounded up
      //     to next page multiple.
      // 2a. If MEMORY_GROWTH_LINEAR_STEP == -1, excessively resize the heap
      //     geometrically: increase the heap size according to
      //     MEMORY_GROWTH_GEOMETRIC_STEP factor (default +20%), At most
      //     overreserve by MEMORY_GROWTH_GEOMETRIC_CAP bytes (default 96MB).
      // 2b. If MEMORY_GROWTH_LINEAR_STEP != -1, excessively resize the heap
      //     linearly: increase the heap size by at least
      //     MEMORY_GROWTH_LINEAR_STEP bytes.
      // 3.  Max size for the heap is capped at 2048MB-WASM_PAGE_SIZE, or by
      //     MAXIMUM_MEMORY, or by ASAN limit, depending on which is smallest
      // 4.  If we were unable to allocate as much memory, it may be due to
      //     over-eager decision to excessively reserve due to (3) above.
      //     Hence if an allocation fails, cut down on the amount of excess
      //     growth, in an attempt to succeed to perform a smaller allocation.
  
      // A limit is set for how much we can grow. We should not exceed that
      // (the wasm binary specifies it, so if we tried, we'd fail anyhow).
      var maxHeapSize = getHeapMax();
      if (requestedSize > maxHeapSize) {
        err(`Cannot enlarge memory, requested ${requestedSize} bytes, but the limit is ${maxHeapSize} bytes!`);
        return false;
      }
  
      // Loop through potential heap size increases. If we attempt a too eager
      // reservation that fails, cut down on the attempted size and reserve a
      // smaller bump instead. (max 3 times, chosen somewhat arbitrarily)
      for (var cutDown = 1; cutDown <= 4; cutDown *= 2) {
        var overGrownHeapSize = oldSize * (1 + 0.2 / cutDown); // ensure geometric growth
        // but limit overreserving (default to capping at +96MB overgrowth at most)
        overGrownHeapSize = Math.min(overGrownHeapSize, requestedSize + 100663296 );
  
        var newSize = Math.min(maxHeapSize, alignMemory(Math.max(requestedSize, overGrownHeapSize), 65536));
  
        var t0 = _emscripten_get_now();
        var replacement = growMemory(newSize);
        var t1 = _emscripten_get_now();
        dbg(`Heap resize call from ${oldSize} to ${newSize} took ${(t1 - t0)} msecs. Success: ${!!replacement}`);
        if (replacement) {
  
          return true;
        }
      }
      err(`Failed to grow the heap from ${oldSize} bytes to ${newSize} bytes, not enough memory!`);
      return false;
    };

  var ENV = {
  };
  
  var getExecutableName = () => thisProgram || './this.program';
  var getEnvStrings = () => {
      if (!getEnvStrings.strings) {
        // Default values.
        // Browser language detection #8751
        var lang = ((typeof navigator == 'object' && navigator.languages && navigator.languages[0]) || 'C').replace('-', '_') + '.UTF-8';
        var env = {
          'USER': 'web_user',
          'LOGNAME': 'web_user',
          'PATH': '/',
          'PWD': '/',
          'HOME': '/home/web_user',
          'LANG': lang,
          '_': getExecutableName()
        };
        // Apply the user-provided values, if any.
        for (var x in ENV) {
          // x is a key in ENV; if ENV[x] is undefined, that means it was
          // explicitly set to be so. We allow user code to do that to
          // force variables with default values to remain unset.
          if (ENV[x] === undefined) delete env[x];
          else env[x] = ENV[x];
        }
        var strings = [];
        for (var x in env) {
          strings.push(`${x}=${env[x]}`);
        }
        getEnvStrings.strings = strings;
      }
      return getEnvStrings.strings;
    };
  
  var stringToAscii = (str, buffer) => {
      for (var i = 0; i < str.length; ++i) {
        assert(str.charCodeAt(i) === (str.charCodeAt(i) & 0xff));
        HEAP8[buffer++] = str.charCodeAt(i);checkInt8(str.charCodeAt(i));
      }
      // Null-terminate the string
      HEAP8[buffer] = 0;checkInt8(0);
    };
  var _environ_get = (__environ, environ_buf) => {
      var bufSize = 0;
      getEnvStrings().forEach((string, i) => {
        var ptr = environ_buf + bufSize;
        HEAPU32[(((__environ)+(i*4))>>2)] = ptr;checkInt32(ptr);
        stringToAscii(string, ptr);
        bufSize += string.length + 1;
      });
      return 0;
    };

  var _environ_sizes_get = (penviron_count, penviron_buf_size) => {
      var strings = getEnvStrings();
      HEAPU32[((penviron_count)>>2)] = strings.length;checkInt32(strings.length);
      var bufSize = 0;
      strings.forEach((string) => bufSize += string.length + 1);
      HEAPU32[((penviron_buf_size)>>2)] = bufSize;checkInt32(bufSize);
      return 0;
    };

  function _fd_close(fd) {
  try {
  
      var stream = SYSCALLS.getStreamFromFD(fd);
      FS.close(stream);
      return 0;
    } catch (e) {
    if (typeof FS == 'undefined' || !(e.name === 'ErrnoError')) throw e;
    return e.errno;
  }
  }

  function _fd_fdstat_get(fd, pbuf) {
  try {
  
      var rightsBase = 0;
      var rightsInheriting = 0;
      var flags = 0;
      {
        var stream = SYSCALLS.getStreamFromFD(fd);
        // All character devices are terminals (other things a Linux system would
        // assume is a character device, like the mouse, we have special APIs for).
        var type = stream.tty ? 2 :
                   FS.isDir(stream.mode) ? 3 :
                   FS.isLink(stream.mode) ? 7 :
                   4;
      }
      HEAP8[pbuf] = type;checkInt8(type);
      HEAP16[(((pbuf)+(2))>>1)] = flags;checkInt16(flags);
      HEAP64[(((pbuf)+(8))>>3)] = BigInt(rightsBase);checkInt64(rightsBase);
      HEAP64[(((pbuf)+(16))>>3)] = BigInt(rightsInheriting);checkInt64(rightsInheriting);
      return 0;
    } catch (e) {
    if (typeof FS == 'undefined' || !(e.name === 'ErrnoError')) throw e;
    return e.errno;
  }
  }

  /** @param {number=} offset */
  var doReadv = (stream, iov, iovcnt, offset) => {
      var ret = 0;
      for (var i = 0; i < iovcnt; i++) {
        var ptr = HEAPU32[((iov)>>2)];
        var len = HEAPU32[(((iov)+(4))>>2)];
        iov += 8;
        var curr = FS.read(stream, HEAP8, ptr, len, offset);
        if (curr < 0) return -1;
        ret += curr;
        if (curr < len) break; // nothing more to read
        if (typeof offset != 'undefined') {
          offset += curr;
        }
      }
      return ret;
    };
  
  function _fd_read(fd, iov, iovcnt, pnum) {
  try {
  
      var stream = SYSCALLS.getStreamFromFD(fd);
      var num = doReadv(stream, iov, iovcnt);
      HEAPU32[((pnum)>>2)] = num;checkInt32(num);
      return 0;
    } catch (e) {
    if (typeof FS == 'undefined' || !(e.name === 'ErrnoError')) throw e;
    return e.errno;
  }
  }

  
  function _fd_seek(fd, offset, whence, newOffset) {
    offset = bigintToI53Checked(offset);
  
    
  try {
  
      if (isNaN(offset)) return 61;
      var stream = SYSCALLS.getStreamFromFD(fd);
      FS.llseek(stream, offset, whence);
      HEAP64[((newOffset)>>3)] = BigInt(stream.position);checkInt64(stream.position);
      if (stream.getdents && offset === 0 && whence === 0) stream.getdents = null; // reset readdir state
      return 0;
    } catch (e) {
    if (typeof FS == 'undefined' || !(e.name === 'ErrnoError')) throw e;
    return e.errno;
  }
  ;
  }

  function _fd_sync(fd) {
  try {
  
      var stream = SYSCALLS.getStreamFromFD(fd);
      if (stream.stream_ops?.fsync) {
        return stream.stream_ops.fsync(stream);
      }
      return 0; // we can't do anything synchronously; the in-memory FS is already synced to
    } catch (e) {
    if (typeof FS == 'undefined' || !(e.name === 'ErrnoError')) throw e;
    return e.errno;
  }
  }

  /** @param {number=} offset */
  var doWritev = (stream, iov, iovcnt, offset) => {
      var ret = 0;
      for (var i = 0; i < iovcnt; i++) {
        var ptr = HEAPU32[((iov)>>2)];
        var len = HEAPU32[(((iov)+(4))>>2)];
        iov += 8;
        var curr = FS.write(stream, HEAP8, ptr, len, offset);
        if (curr < 0) return -1;
        ret += curr;
        if (curr < len) {
          // No more space to write.
          break;
        }
        if (typeof offset != 'undefined') {
          offset += curr;
        }
      }
      return ret;
    };
  
  function _fd_write(fd, iov, iovcnt, pnum) {
  try {
  
      var stream = SYSCALLS.getStreamFromFD(fd);
      var num = doWritev(stream, iov, iovcnt);
      HEAPU32[((pnum)>>2)] = num;checkInt32(num);
      return 0;
    } catch (e) {
    if (typeof FS == 'undefined' || !(e.name === 'ErrnoError')) throw e;
    return e.errno;
  }
  }

  FS.createPreloadedFile = FS_createPreloadedFile;
  FS.staticInit();
  // Set module methods based on EXPORTED_RUNTIME_METHODS
  ;
function checkIncomingModuleAPI() {
  ignoredModuleProp('fetchSettings');
}
var wasmImports = {
  /** @export */
  __assert_fail: ___assert_fail,
  /** @export */
  __handle_stack_overflow: ___handle_stack_overflow,
  /** @export */
  __syscall_chmod: ___syscall_chmod,
  /** @export */
  __syscall_faccessat: ___syscall_faccessat,
  /** @export */
  __syscall_fchmod: ___syscall_fchmod,
  /** @export */
  __syscall_fchown32: ___syscall_fchown32,
  /** @export */
  __syscall_fcntl64: ___syscall_fcntl64,
  /** @export */
  __syscall_fstat64: ___syscall_fstat64,
  /** @export */
  __syscall_ftruncate64: ___syscall_ftruncate64,
  /** @export */
  __syscall_getcwd: ___syscall_getcwd,
  /** @export */
  __syscall_ioctl: ___syscall_ioctl,
  /** @export */
  __syscall_lstat64: ___syscall_lstat64,
  /** @export */
  __syscall_mkdirat: ___syscall_mkdirat,
  /** @export */
  __syscall_newfstatat: ___syscall_newfstatat,
  /** @export */
  __syscall_openat: ___syscall_openat,
  /** @export */
  __syscall_readlinkat: ___syscall_readlinkat,
  /** @export */
  __syscall_rmdir: ___syscall_rmdir,
  /** @export */
  __syscall_stat64: ___syscall_stat64,
  /** @export */
  __syscall_unlinkat: ___syscall_unlinkat,
  /** @export */
  __syscall_utimensat: ___syscall_utimensat,
  /** @export */
  _abort_js: __abort_js,
  /** @export */
  _localtime_js: __localtime_js,
  /** @export */
  _mmap_js: __mmap_js,
  /** @export */
  _munmap_js: __munmap_js,
  /** @export */
  _tzset_js: __tzset_js,
  /** @export */
  clock_time_get: _clock_time_get,
  /** @export */
  emscripten_date_now: _emscripten_date_now,
  /** @export */
  emscripten_get_heap_max: _emscripten_get_heap_max,
  /** @export */
  emscripten_get_now: _emscripten_get_now,
  /** @export */
  emscripten_resize_heap: _emscripten_resize_heap,
  /** @export */
  environ_get: _environ_get,
  /** @export */
  environ_sizes_get: _environ_sizes_get,
  /** @export */
  fd_close: _fd_close,
  /** @export */
  fd_fdstat_get: _fd_fdstat_get,
  /** @export */
  fd_read: _fd_read,
  /** @export */
  fd_seek: _fd_seek,
  /** @export */
  fd_sync: _fd_sync,
  /** @export */
  fd_write: _fd_write,
  /** @export */
  memory: wasmMemory
};
var wasmExports;
createWasm();
var ___wasm_call_ctors = createExportWrapper('__wasm_call_ctors', 0);
var _strerror = createExportWrapper('strerror', 1);
var _malloc = Module['_malloc'] = createExportWrapper('malloc', 1);
var _free = Module['_free'] = createExportWrapper('free', 1);
var _realloc = Module['_realloc'] = createExportWrapper('realloc', 2);
var _fflush = createExportWrapper('fflush', 1);
var _emscripten_builtin_memalign = createExportWrapper('emscripten_builtin_memalign', 2);
var _emscripten_stack_init = () => (_emscripten_stack_init = wasmExports['emscripten_stack_init'])();
var _emscripten_stack_get_free = () => (_emscripten_stack_get_free = wasmExports['emscripten_stack_get_free'])();
var _emscripten_stack_get_base = () => (_emscripten_stack_get_base = wasmExports['emscripten_stack_get_base'])();
var _emscripten_stack_get_end = () => (_emscripten_stack_get_end = wasmExports['emscripten_stack_get_end'])();
var __emscripten_stack_restore = (a0) => (__emscripten_stack_restore = wasmExports['_emscripten_stack_restore'])(a0);
var __emscripten_stack_alloc = (a0) => (__emscripten_stack_alloc = wasmExports['_emscripten_stack_alloc'])(a0);
var _emscripten_stack_get_current = () => (_emscripten_stack_get_current = wasmExports['emscripten_stack_get_current'])();
var ___set_stack_limits = Module['___set_stack_limits'] = createExportWrapper('__set_stack_limits', 2);


// include: postamble.js
// === Auto-generated postamble setup entry stuff ===

Module['wasmMemory'] = wasmMemory;
var missingLibrarySymbols = [
  'writeI53ToI64',
  'writeI53ToI64Clamped',
  'writeI53ToI64Signaling',
  'writeI53ToU64Clamped',
  'writeI53ToU64Signaling',
  'readI53FromU64',
  'convertI32PairToI53',
  'convertI32PairToI53Checked',
  'convertU32PairToI53',
  'stackAlloc',
  'getTempRet0',
  'setTempRet0',
  'exitJS',
  'inetPton4',
  'inetNtop4',
  'inetPton6',
  'inetNtop6',
  'readSockaddr',
  'writeSockaddr',
  'emscriptenLog',
  'readEmAsmArgs',
  'jstoi_q',
  'listenOnce',
  'autoResumeAudioContext',
  'getDynCaller',
  'dynCall',
  'handleException',
  'keepRuntimeAlive',
  'runtimeKeepalivePush',
  'runtimeKeepalivePop',
  'callUserCallback',
  'maybeExit',
  'asmjsMangle',
  'HandleAllocator',
  'getNativeTypeSize',
  'STACK_SIZE',
  'STACK_ALIGN',
  'POINTER_SIZE',
  'ASSERTIONS',
  'getCFunc',
  'ccall',
  'cwrap',
  'uleb128Encode',
  'sigToWasmTypes',
  'generateFuncType',
  'convertJsFunctionToWasm',
  'getEmptyTableSlot',
  'updateTableMap',
  'getFunctionAddress',
  'addFunction',
  'removeFunction',
  'reallyNegative',
  'unSign',
  'strLen',
  'reSign',
  'formatString',
  'intArrayToString',
  'AsciiToString',
  'UTF16ToString',
  'stringToUTF16',
  'lengthBytesUTF16',
  'UTF32ToString',
  'stringToUTF32',
  'lengthBytesUTF32',
  'stringToNewUTF8',
  'stringToUTF8OnStack',
  'writeArrayToMemory',
  'registerKeyEventCallback',
  'maybeCStringToJsString',
  'findEventTarget',
  'getBoundingClientRect',
  'fillMouseEventData',
  'registerMouseEventCallback',
  'registerWheelEventCallback',
  'registerUiEventCallback',
  'registerFocusEventCallback',
  'fillDeviceOrientationEventData',
  'registerDeviceOrientationEventCallback',
  'fillDeviceMotionEventData',
  'registerDeviceMotionEventCallback',
  'screenOrientation',
  'fillOrientationChangeEventData',
  'registerOrientationChangeEventCallback',
  'fillFullscreenChangeEventData',
  'registerFullscreenChangeEventCallback',
  'JSEvents_requestFullscreen',
  'JSEvents_resizeCanvasForFullscreen',
  'registerRestoreOldStyle',
  'hideEverythingExceptGivenElement',
  'restoreHiddenElements',
  'setLetterbox',
  'softFullscreenResizeWebGLRenderTarget',
  'doRequestFullscreen',
  'fillPointerlockChangeEventData',
  'registerPointerlockChangeEventCallback',
  'registerPointerlockErrorEventCallback',
  'requestPointerLock',
  'fillVisibilityChangeEventData',
  'registerVisibilityChangeEventCallback',
  'registerTouchEventCallback',
  'fillGamepadEventData',
  'registerGamepadEventCallback',
  'registerBeforeUnloadEventCallback',
  'fillBatteryEventData',
  'battery',
  'registerBatteryEventCallback',
  'setCanvasElementSize',
  'getCanvasElementSize',
  'jsStackTrace',
  'getCallstack',
  'convertPCtoSourceLocation',
  'wasiRightsToMuslOFlags',
  'wasiOFlagsToMuslOFlags',
  'safeSetTimeout',
  'setImmediateWrapped',
  'safeRequestAnimationFrame',
  'clearImmediateWrapped',
  'polyfillSetImmediate',
  'registerPostMainLoop',
  'registerPreMainLoop',
  'getPromise',
  'makePromise',
  'idsToPromises',
  'makePromiseCallback',
  'ExceptionInfo',
  'findMatchingCatch',
  'Browser_asyncPrepareDataCounter',
  'arraySum',
  'addDays',
  'getSocketFromFD',
  'getSocketAddress',
  'FS_unlink',
  'FS_mkdirTree',
  '_setNetworkCallback',
  'heapObjectForWebGLType',
  'toTypedArrayIndex',
  'webgl_enable_ANGLE_instanced_arrays',
  'webgl_enable_OES_vertex_array_object',
  'webgl_enable_WEBGL_draw_buffers',
  'webgl_enable_WEBGL_multi_draw',
  'webgl_enable_EXT_polygon_offset_clamp',
  'webgl_enable_EXT_clip_control',
  'webgl_enable_WEBGL_polygon_mode',
  'emscriptenWebGLGet',
  'computeUnpackAlignedImageSize',
  'colorChannelsInGlTextureFormat',
  'emscriptenWebGLGetTexPixelData',
  'emscriptenWebGLGetUniform',
  'webglGetUniformLocation',
  'webglPrepareUniformLocationsBeforeFirstUse',
  'webglGetLeftBracePos',
  'emscriptenWebGLGetVertexAttrib',
  '__glGetActiveAttribOrUniform',
  'writeGLArray',
  'registerWebGlEventCallback',
  'runAndAbortIfError',
  'ALLOC_NORMAL',
  'ALLOC_STACK',
  'allocate',
  'writeStringToMemory',
  'writeAsciiToMemory',
  'setErrNo',
  'demangle',
  'stackTrace',
];
missingLibrarySymbols.forEach(missingLibrarySymbol)

var unexportedSymbols = [
  'run',
  'addOnPreRun',
  'addOnInit',
  'addOnPreMain',
  'addOnExit',
  'addOnPostRun',
  'addRunDependency',
  'removeRunDependency',
  'out',
  'err',
  'callMain',
  'abort',
  'wasmExports',
  'writeStackCookie',
  'checkStackCookie',
  'readI53FromI64',
  'INT53_MAX',
  'INT53_MIN',
  'bigintToI53Checked',
  'stackSave',
  'stackRestore',
  'ptrToString',
  'zeroMemory',
  'getHeapMax',
  'growMemory',
  'ENV',
  'setStackLimits',
  'ERRNO_CODES',
  'strError',
  'DNS',
  'Protocols',
  'Sockets',
  'timers',
  'warnOnce',
  'readEmAsmArgsArray',
  'jstoi_s',
  'getExecutableName',
  'asyncLoad',
  'alignMemory',
  'mmapAlloc',
  'wasmTable',
  'noExitRuntime',
  'freeTableIndexes',
  'functionsInTableMap',
  'setValue',
  'getValue',
  'PATH',
  'PATH_FS',
  'UTF8Decoder',
  'UTF8ArrayToString',
  'UTF8ToString',
  'stringToUTF8Array',
  'stringToUTF8',
  'lengthBytesUTF8',
  'intArrayFromString',
  'stringToAscii',
  'UTF16Decoder',
  'JSEvents',
  'specialHTMLTargets',
  'findCanvasEventTarget',
  'currentFullscreenStrategy',
  'restoreOldWindowedStyle',
  'UNWIND_CACHE',
  'ExitStatus',
  'getEnvStrings',
  'checkWasiClock',
  'doReadv',
  'doWritev',
  'initRandomFill',
  'randomFill',
  'promiseMap',
  'uncaughtExceptionCount',
  'exceptionLast',
  'exceptionCaught',
  'Browser',
  'getPreloadedImageData__data',
  'wget',
  'MONTH_DAYS_REGULAR',
  'MONTH_DAYS_LEAP',
  'MONTH_DAYS_REGULAR_CUMULATIVE',
  'MONTH_DAYS_LEAP_CUMULATIVE',
  'isLeapYear',
  'ydayFromDate',
  'SYSCALLS',
  'preloadPlugins',
  'FS_createPreloadedFile',
  'FS_modeStringToFlags',
  'FS_getMode',
  'FS_stdin_getChar_buffer',
  'FS_stdin_getChar',
  'FS_createPath',
  'FS_createDevice',
  'FS_readFile',
  'FS',
  'FS_createDataFile',
  'FS_createLazyFile',
  'MEMFS',
  'TTY',
  'PIPEFS',
  'SOCKFS',
  'tempFixedLengthArray',
  'miniTempWebGLFloatBuffers',
  'miniTempWebGLIntBuffers',
  'GL',
  'AL',
  'GLUT',
  'EGL',
  'GLEW',
  'IDBStore',
  'SDL',
  'SDL_gfx',
  'allocateUTF8',
  'allocateUTF8OnStack',
  'print',
  'printErr',
];
unexportedSymbols.forEach(unexportedRuntimeSymbol);



var calledRun;

dependenciesFulfilled = function runCaller() {
  // If run has never been called, and we should call run (INVOKE_RUN is true, and Module.noInitialRun is not false)
  if (!calledRun) run();
  if (!calledRun) dependenciesFulfilled = runCaller; // try this again later, after new deps are fulfilled
};

function stackCheckInit() {
  // This is normally called automatically during __wasm_call_ctors but need to
  // get these values before even running any of the ctors so we call it redundantly
  // here.
  _emscripten_stack_init();
  // TODO(sbc): Move writeStackCookie to native to to avoid this.
  writeStackCookie();
}

function run() {

  if (runDependencies > 0) {
    return;
  }

  stackCheckInit();

  preRun();

  // a preRun added a dependency, run will be called later
  if (runDependencies > 0) {
    return;
  }

  function doRun() {
    // run may have just been called through dependencies being fulfilled just in this very frame,
    // or while the async setStatus time below was happening
    if (calledRun) return;
    calledRun = true;
    Module['calledRun'] = true;

    if (ABORT) return;

    initRuntime();

    readyPromiseResolve(Module);
    Module['onRuntimeInitialized']?.();

    assert(!Module['_main'], 'compiled without a main, but one is present. if you added it from JS, use Module["onRuntimeInitialized"]');

    postRun();
  }

  if (Module['setStatus']) {
    Module['setStatus']('Running...');
    setTimeout(() => {
      setTimeout(() => Module['setStatus'](''), 1);
      doRun();
    }, 1);
  } else
  {
    doRun();
  }
  checkStackCookie();
}

function checkUnflushedContent() {
  // Compiler settings do not allow exiting the runtime, so flushing
  // the streams is not possible. but in ASSERTIONS mode we check
  // if there was something to flush, and if so tell the user they
  // should request that the runtime be exitable.
  // Normally we would not even include flush() at all, but in ASSERTIONS
  // builds we do so just for this check, and here we see if there is any
  // content to flush, that is, we check if there would have been
  // something a non-ASSERTIONS build would have not seen.
  // How we flush the streams depends on whether we are in SYSCALLS_REQUIRE_FILESYSTEM=0
  // mode (which has its own special function for this; otherwise, all
  // the code is inside libc)
  var oldOut = out;
  var oldErr = err;
  var has = false;
  out = err = (x) => {
    has = true;
  }
  try { // it doesn't matter if it fails
    _fflush(0);
    // also flush in the JS FS layer
    ['stdout', 'stderr'].forEach((name) => {
      var info = FS.analyzePath('/dev/' + name);
      if (!info) return;
      var stream = info.object;
      var rdev = stream.rdev;
      var tty = TTY.ttys[rdev];
      if (tty?.output?.length) {
        has = true;
      }
    });
  } catch(e) {}
  out = oldOut;
  err = oldErr;
  if (has) {
    warnOnce('stdio streams had content in them that was not flushed. you should set EXIT_RUNTIME to 1 (see the Emscripten FAQ), or make sure to emit a newline when you printf etc.');
  }
}

if (Module['preInit']) {
  if (typeof Module['preInit'] == 'function') Module['preInit'] = [Module['preInit']];
  while (Module['preInit'].length > 0) {
    Module['preInit'].pop()();
  }
}

run();

// end include: postamble.js

// include: /src/dist/.build/sqlite-src-3530300/ext/wasm/bld/post-js.esm.js
/**
   post-js-header.js is to be prepended to other code to create
   post-js.js for use with Emscripten's --post-js flag, so it gets
   injected in the earliest stages of sqlite3InitModule().

   Running this function will bootstrap the library and return
   a Promise to the sqlite3 namespace object.

   In the canonical builds, this gets called by extern-post-js.c-pp.js
*/
Module.runSQLite3PostLoadInit = async function(
  sqlite3InitScriptInfo,
  EmscriptenModule/*the Emscripten-style module object*/,
  sqlite3IsUnderTest
){
  /** ^^^ Don't use Module.postRun, as that runs a different time
      depending on whether this file is built with emcc 3.1.x or
      4.0.x. This function name is intentionally obnoxiously verbose to
      ensure that we don't collide with current and future Emscripten
      symbol names. */
  'use strict';
  delete EmscriptenModule.runSQLite3PostLoadInit;
  //console.warn("This is the start of Module.runSQLite3PostLoadInit()");
  /* This function will contain at least the following:

     - post-js-header.js          => this file
       - sqlite3-api-prologue.js  => Bootstrapping bits for the following files
       - common/whwasmutil.js     => Generic JS/WASM glue
       - jaccwabyt/jaccwabyt.js   => C struct-binding glue
       - sqlite3-api-glue.js      => glues previous parts together
       - sqlite3-api-oo1.js       => SQLite3 OO API #1
       - sqlite3-api-worker1.js   => Worker-based API
       - sqlite3-vfs-helper.c-pp.js  => Utilities for VFS impls
       - sqlite3-vtab-helper.c-pp.js => Utilities for virtual table impls
       - sqlite3-vfs-opfs.c-pp.js => OPFS VFS
       - sqlite3-vfs-opfs-sahpool.c-pp.js => OPFS SAHPool VFS
       - sqlite3-vfs-opfs-wl.c-pp.js => WebLock-using OPFS VFS
     - post-js-footer.js          => this file's epilogue

     And all of that gets sandwiched between extern-pre-js.js and
     extern-post-js.js.
  */
/* @preserve
**
** LICENSE for the sqlite3 WebAssembly/JavaScript APIs.
**
** This bundle (typically released as sqlite3.js or sqlite3.mjs)
** is an amalgamation of JavaScript source code from two projects:
**
** 1) https://emscripten.org: the Emscripten "glue code" is covered by
**    the terms of the MIT license and University of Illinois/NCSA
**    Open Source License, as described at:
**
**    https://emscripten.org/docs/introducing_emscripten/emscripten_license.html
**
** 2) https://sqlite.org: all code and documentation labeled as being
**    from this source are released under the same terms as the sqlite3
**    C library:
**
** 2022-10-16
**
** The author disclaims copyright to this source code.  In place of a
** legal notice, here is a blessing:
**
** *   May you do good and not evil.
** *   May you find forgiveness for yourself and forgive others.
** *   May you share freely, never taking more than you give.
*/
/* @preserve
** This code was built from sqlite3 version...
**
** SQLITE_VERSION "3.53.3"
** SQLITE_VERSION_NUMBER 3053003
** SQLITE_SOURCE_ID "2026-06-26 20:14:12 d4c0e51e4aeb96955b99185ab9cde75c339e2c29c3f3f12428d364a10d782c62"
**
** Emscripten SDK: 3.1.74
*/
/*
  2022-05-22

  The author disclaims copyright to this source code.  In place of a
  legal notice, here is a blessing:

  *   May you do good and not evil.
  *   May you find forgiveness for yourself and forgive others.
  *   May you share freely, never taking more than you give.

  ***********************************************************************

  This file is intended to be combined at build-time with other
  related code, most notably a header and footer which wraps this
  whole file into a single callback which can be run after Emscripten
  loads the corresponding WASM module. The sqlite3 JS API has no hard
  requirements on Emscripten and does not expose any Emscripten APIs
  to clients. It is structured such that its build can be tweaked to
  include it in arbitrary WASM environments which can supply the
  necessary underlying features (e.g. a POSIX file I/O layer).

  Main project home page: https://sqlite.org

  Documentation home page: https://sqlite.org/wasm
*/

/**
   sqlite3ApiBootstrap() is the only global symbol persistently
   exposed by this API. It is intended to be called one time at the
   end of the API amalgamation process and passed configuration details
   for the current environment.

   This function is not intended for client-level use. It is intended
   for use in creating bundles configured for specific WASM
   environments. That said, the "sqlite3-api.js" intermediary build
   file aims to be suitable for dropping in to custom builds, and it
   exposes only this function.

   This function expects a configuration object, intended to abstract
   away details specific to any given WASM environment, primarily so
   that it can be used without any direct dependency on
   Emscripten. (Note the default values for the config object!) The
   config object is only honored the first time this is
   called. Subsequent calls ignore the argument and return the same
   (configured) object which gets initialized by the first call.  This
   function will throw if any of the required config options are
   missing.

   The config object properties include:

   - `exports`[^1]: the "exports" object for the current WASM
     environment. In an Emscripten-based build, this should be set to
     `Module['asm']` (versions <=3.1.43) or `Module['wasmExports']`
     (versions >=3.1.44).

   - `memory`[^1]: optional WebAssembly.Memory object, defaulting to
     `exports.memory`. In Emscripten environments this should be set
     to `Module.wasmMemory` if the build uses `-sIMPORTED_MEMORY`, or be
     left undefined/falsy to default to `exports.memory` when using
     WASM-exported memory.

   - `functionTable`[^1]: optional WebAssembly.Table object holding
     the indirect function table. If not set then the table is assumed
     to be in `exports.__indirect_function_table`.

   - `bigIntEnabled`: true if BigInt support is enabled. Defaults to
     true if `globalThis.BigInt64Array` is available, else false. Some APIs
     will throw exceptions if called without BigInt support, as BigInt
     is required for marshalling C-side int64 into and out of JS.
     (Sidebar: it is technically possible to add int64 support via
     marshalling of int32 pairs, but doing so is unduly invasive.)

   - `allocExportName`: the name of the function, in `exports`, of the
     `malloc(3)`-compatible routine for the WASM environment. Defaults
     to `"sqlite3_malloc"`. Beware that using any allocator other than
     sqlite3_malloc() may require care in certain client-side code
     regarding which allocator is uses. Notably, sqlite3_deserialize()
     and sqlite3_serialize() can only safely use memory from different
     allocators under very specific conditions. The canonical builds
     of this API guaranty that `sqlite3_malloc()` is the JS-side
     allocator implementation.

   - `deallocExportName`: the name of the function, in `exports`, of
     the `free(3)`-compatible routine for the WASM
     environment. Defaults to `"sqlite3_free"`.

   - `reallocExportName`: the name of the function, in `exports`, of
     the `realloc(3)`-compatible routine for the WASM
     environment. Defaults to `"sqlite3_realloc"`.

   - `debug`, `log`, `warn`, and `error` may be functions equivalent
     to the like-named methods of the global `console` object. By
     default, these map directly to their `console` counterparts, but
     can be replaced with (e.g.) empty functions to squelch all such
     output.

   - `wasmfsOpfsDir`[^1]: Specifies the "mount point" of the
     OPFS-backed filesystem in WASMFS-capable builds. This is only
     used in WASMFS-capable builds of the library (which the canonical
     builds do not include).

     - `disable` (as of 3.53.0) may be an object with the following
     properties:
       - `vfs`, an object, may contain a map of VFS names to booleans.
       Any mapping to falsy are disabled. The supported names
       are: "kvvfs", "opfs", "opfs-sahpool", "opfs-wl".
       - Other disabling options may be added in the future.

   [^1] = This property may optionally be a function, in which case
          this function calls that function to fetch the value,
          enabling delayed evaluation.

   The returned object is the top-level sqlite3 namespace object.


   Client code may optionally assign sqlite3ApiBootstrap.defaultConfig
   an object-type value before calling sqlite3ApiBootstrap() (without
   arguments) in order to tell that call to use this object as its
   default config value. The intention of this is to provide
   downstream clients with a reasonably flexible approach for plugging
   in an environment-suitable configuration without having to define a
   new global-scope symbol.

   However, because clients who access this library via an
   Emscripten-hosted module will not have an opportunity to call
   sqlite3ApiBootstrap() themselves, nor to access it before it is
   called, an alternative option for setting the configuration is to
   define globalThis.sqlite3ApiConfig to an object. If it is set, it
   is used instead of sqlite3ApiBootstrap.defaultConfig if
   sqlite3ApiBootstrap() is called without arguments. Setting the
   `exports` and `memory` parts require already having loaded the WASM
   module, though.

   Both sqlite3ApiBootstrap.defaultConfig and
   globalThis.sqlite3ApiConfig get deleted by sqlite3ApiBootstrap()
   because any changes to them made after that point would have no
   useful effect. This function also deletes itself from globalThis
   when it's called.

   This function returns a Promise to the sqlite3 namespace object,
   which resolves after the async pieces of the library init are
   complete.
*/
'use strict';
globalThis.sqlite3ApiBootstrap = async function sqlite3ApiBootstrap(
  apiConfig = (globalThis.sqlite3ApiConfig || sqlite3ApiBootstrap.defaultConfig)
){
  if(sqlite3ApiBootstrap.sqlite3){ /* already initialized */
    (sqlite3ApiBootstrap.sqlite3.config || console).warn(
      "sqlite3ApiBootstrap() called multiple times.",
      "Config and external initializers are ignored on calls after the first."
    );
    return sqlite3ApiBootstrap.sqlite3;
  }
  const nu = (...obj)=>Object.assign(Object.create(null),...obj);
  const config = nu({
    exports: undefined,
    memory: undefined,
    bigIntEnabled: !!globalThis.BigInt64Array,
    debug: console.debug.bind(console),
    warn: console.warn.bind(console),
    error: console.error.bind(console),
    log: console.log.bind(console),
    wasmfsOpfsDir: '/opfs',
    /**
       useStdAlloc is just for testing allocator discrepancies. The
       docs guarantee that this is false in the canonical builds. For
       99% of purposes it doesn't matter which allocators we use, but
       it becomes significant with, e.g., sqlite3_deserialize() and
       certain wasm.xWrap.resultAdapter()s.
    */
    useStdAlloc: false
  }, apiConfig);

  Object.assign(config, {
    allocExportName: config.useStdAlloc ? 'malloc' : 'sqlite3_malloc',
    deallocExportName: config.useStdAlloc ? 'free' : 'sqlite3_free',
    reallocExportName: config.useStdAlloc ? 'realloc' : 'sqlite3_realloc'
  });

  [
    // If any of these config options are functions, replace them with
    // the result of calling that function. They must not be async.
    'exports', 'memory', 'functionTable', 'wasmfsOpfsDir'
  ].forEach((k)=>{
    if('function' === typeof config[k]){
      config[k] = config[k]();
    }
  });

  /**
      The main sqlite3 binding API gets installed into this object,
      mimicking the C API as closely as we can. The numerous members
      names with prefixes 'sqlite3_' and 'SQLITE_' behave, insofar as
      possible, identically to the C-native counterparts, as documented at:

      https://sqlite.org/c3ref/intro.html

      A very few exceptions require an additional level of proxy
      function or may otherwise require special attention in the WASM
      environment, and all such cases are documented somewhere below
      in this file or in sqlite3-api-glue.js. capi members which are
      not documented are installed as 1-to-1 proxies for their
      C-side counterparts.
  */
  const capi = nu();
  /**
     Holds state which are specific to the WASM-related
     infrastructure and glue code.

     Note that a number of members of this object are injected
     dynamically after the api object is fully constructed, so
     not all are documented in this file.
  */
  const wasm = nu();

  /** Internal helper for SQLite3Error ctor. */
  const __rcStr = (rc)=>{
    return (capi.sqlite3_js_rc_str && capi.sqlite3_js_rc_str(rc))
           || ("Unknown result code #"+rc);
  };

  /** Internal helper for SQLite3Error ctor. */
  const isInt32 = (n)=>
        'number'===typeof n
        && n===(n | 0)
        && n<=2147483647 && n>=-2147483648;

  /**
     An Error subclass specifically for reporting DB-level errors and
     enabling clients to unambiguously identify such exceptions.
     The C-level APIs never throw, but some of the higher-level
     C-style APIs do and the object-oriented APIs use exceptions
     exclusively to report errors.
  */
  class SQLite3Error extends Error {
    /**
       Constructs this object with a message depending on its arguments:

       If its first argument is an integer, it is assumed to be
       an SQLITE_... result code and it is passed to
       sqlite3.capi.sqlite3_js_rc_str() to stringify it.

       If called with exactly 2 arguments and the 2nd is an object,
       that object is treated as the 2nd argument to the parent
       constructor.

       The exception's message is created by concatenating its
       arguments with a space between each, except for the
       two-args-with-an-object form and that the first argument will
       get coerced to a string, as described above, if it's an
       integer.

       If passed an integer first argument, the error object's
       `resultCode` member will be set to the given integer value,
       else it will be set to capi.SQLITE_ERROR.
    */
    constructor(...args){
      let rc;
      if(args.length){
        if(isInt32(args[0])){
          rc = args[0];
          if(1===args.length){
            super(__rcStr(args[0]));
          }else{
            const rcStr = __rcStr(rc);
            if('object'===typeof args[1]){
              super(rcStr,args[1]);
            }else{
              args[0] = rcStr+':';
              super(args.join(' '));
            }
          }
        }else{
          if(2===args.length && 'object'===typeof args[1]){
            super(...args);
          }else{
            super(args.join(' '));
          }
        }
      }
      this.resultCode = rc || capi.SQLITE_ERROR;
      this.name = 'SQLite3Error';
    }
  };

  /**
     Functionally equivalent to the SQLite3Error constructor but may
     be used as part of an expression, e.g.:

     ```
     return someFunction(x) || SQLite3Error.toss(...);
     ```
  */
  SQLite3Error.toss = (...args)=>{
    throw new SQLite3Error(...args);
  };
  const toss3 = SQLite3Error.toss;

  if(config.wasmfsOpfsDir && !/^\/[^/]+$/.test(config.wasmfsOpfsDir)){
    toss3("config.wasmfsOpfsDir must be falsy or in the form '/dir-name'.");
  }

  /**
     Returns true if the given BigInt value is small enough to fit
     into an int64 value, else false.
  */
  const bigIntFits64 = function f(b){
    if(!f._max){
      f._max = BigInt("0x7fffffffffffffff");
      f._min = ~f._max;
    }
    return b >= f._min && b <= f._max;
  };

  /**
     Returns true if the given BigInt value is small enough to fit
     into an int32, else false.
  */
  const bigIntFits32 = (b)=>(b >= (-0x7fffffffn - 1n) && b <= 0x7fffffffn);

  /**
     Returns true if the given BigInt value is small enough to fit
     into a double value without loss of precision, else false.
  */
  const bigIntFitsDouble = function f(b){
    if(!f._min){
      f._min = Number.MIN_SAFE_INTEGER;
      f._max = Number.MAX_SAFE_INTEGER;
    }
    return b >= f._min && b <= f._max;
  };

  /** Returns v if v appears to be a TypedArray, else false. */
  const isTypedArray = (v)=>{
    return (v && v.constructor && isInt32(v.constructor.BYTES_PER_ELEMENT)) ? v : false;
  };

  /**
     Returns true if v appears to be one of our bind()-able TypedArray
     types: Uint8Array or Int8Array or ArrayBuffer. Support for
     TypedArrays with element sizes >1 is a potential TODO just
     waiting on a use case to justify them. Until then, their `buffer`
     property can be used to pass them as an ArrayBuffer. If it's not
     a bindable array type, a falsy value is returned.
  */
  const isBindableTypedArray = (v)=>
        v && (v instanceof Uint8Array
              || v instanceof Int8Array
              || v instanceof ArrayBuffer);

  /**
     Returns true if v appears to be one of the TypedArray types
     which is legal for holding SQL code (as opposed to binary blobs).

     Currently this is the same as isBindableTypedArray() but it
     seems likely that we'll eventually want to add Uint32Array
     and friends to the isBindableTypedArray() list but not to the
     isSQLableTypedArray() list.
  */
  const isSQLableTypedArray = (v)=>
        v && (v instanceof Uint8Array
              || v instanceof Int8Array
              || v instanceof ArrayBuffer);

  /** Returns true if isBindableTypedArray(v) does, else throws with a message
      that v is not a supported TypedArray value. */
  const affirmBindableTypedArray = (v)=>
        isBindableTypedArray(v)
        || toss3("Value is not of a supported TypedArray type.");

  /**
     If v is-a Array, its join("") result is returned.  If
     isSQLableTypedArray(v) is true then wasm.typedArrayToString(v) is
     returned. If it looks like a WASM pointer, wasm.cstrToJs(v) is
     returned. Else v is returned as-is.

     Reminder to self: the "return as-is" instead of returning ''+v is
     arguably a design mistake but changing it is risky at this point.
  */
  const flexibleString = function(v){
    if(isSQLableTypedArray(v)){
      return wasm.typedArrayToString(
        (v instanceof ArrayBuffer) ? new Uint8Array(v) : v,
        0, v.length
      );
    }
    else if(Array.isArray(v)) return v.join("");
    else if(wasm.isPtr(v)) v = wasm.cstrToJs(v);
    return v;
  };

  /**
     An Error subclass specifically for reporting Wasm-level malloc()
     failure and enabling clients to unambiguously identify such
     exceptions.
  */
  class WasmAllocError extends Error {
    /**
       If called with 2 arguments and the 2nd one is an object, it
       behaves like the Error constructor, else it concatenates all
       arguments together with a single space between each to
       construct an error message string. As a special case, if
       called with no arguments then it uses a default error
       message.
    */
    constructor(...args){
      if(2===args.length && 'object'===typeof args[1]){
        super(...args);
      }else if(args.length){
        super(args.join(' '));
      }else{
        super("Allocation failed.");
      }
      this.resultCode = capi.SQLITE_NOMEM;
      this.name = 'WasmAllocError';
    }
  };
  /**
     Functionally equivalent to the WasmAllocError constructor but may
     be used as part of an expression, e.g.:

     ```
     return someAllocatingFunction(x) || WasmAllocError.toss(...);
     ```
  */
  WasmAllocError.toss = (...args)=>{
    throw new WasmAllocError(...args);
  };

  Object.assign(capi, {
    /**
       sqlite3_bind_blob() works exactly like its C counterpart unless
       its 3rd argument is one of:

       - JS string: the 3rd argument is converted to a C string, the
         4th argument is ignored, and the C-string's length is used
         in its place.

       - Array: converted to a string as defined for "flexible
         strings" and then it's treated as a JS string.

       - Int8Array or Uint8Array: wasm.allocFromTypedArray() is used to
         conver the memory to the WASM heap. If the 4th argument is
         0 or greater, it is used as-is, otherwise the array's byteLength
         value is used. This is an exception to the C API's undefined
         behavior for a negative 4th argument, but results are undefined
         if the given 4th argument value is greater than the byteLength
         of the input array.

       - If it's an ArrayBuffer, it gets wrapped in a Uint8Array and
         treated as that type.

       In all of those cases, the final argument (destructor) is
       ignored and capi.SQLITE_WASM_DEALLOC is assumed.

       A 3rd argument of `null` is treated as if it were a WASM pointer
       of 0.

       If the 3rd argument is neither a WASM pointer nor one of the
       above-described types, capi.SQLITE_MISUSE is returned.

       The first argument may be either an `sqlite3_stmt*` WASM
       pointer or an sqlite3.oo1.Stmt instance.

       For consistency with the C API, it requires the same number of
       arguments. It returns capi.SQLITE_MISUSE if passed any other
       argument count.
    */
    sqlite3_bind_blob: undefined/*installed later*/,

    /**
       sqlite3_bind_text() works exactly like its C counterpart unless
       its 3rd argument is one of:

       - JS string: the 3rd argument is converted to a C string, the
         4th argument is ignored, and the C-string's length is used
         in its place.

       - Array: converted to a string as defined for "flexible
         strings". The 4th argument is ignored and a value of -1
         is assumed.

       - Int8Array or Uint8Array: is assumed to contain UTF-8 text, is
         converted to a string. The 4th argument is ignored, replaced
         by the array's byteLength value.

       - If it's an ArrayBuffer, it gets wrapped in a Uint8Array and
         treated as that type.

       In each of those cases, the final argument (text destructor) is
       ignored and capi.SQLITE_WASM_DEALLOC is assumed.

       A 3rd argument of `null` is treated as if it were a WASM pointer
       of 0.

       If the 3rd argument is neither a WASM pointer nor one of the
       above-described types, capi.SQLITE_MISUSE is returned.

       The first argument may be either an `sqlite3_stmt*` WASM
       pointer or an sqlite3.oo1.Stmt instance.

       For consistency with the C API, it requires the same number of
       arguments. It returns capi.SQLITE_MISUSE if passed any other
       argument count.

       If client code needs to bind partial strings, it needs to
       either parcel the string up before passing it in here or it
       must pass in a WASM pointer for the 3rd argument and a valid
       4th-argument value, taking care not to pass a value which
       truncates a multi-byte UTF-8 character. When passing
       WASM-format strings, it is important that the final argument be
       valid or unexpected content can result, or WASM may crash if
       the application reads past the WASM heap bounds.
    */
    sqlite3_bind_text: undefined/*installed later*/,

    /**
       sqlite3_create_function_v2() differs from its native
       counterpart only in the following ways:

       1) The fourth argument (`eTextRep`) argument must not specify
       any encoding other than sqlite3.SQLITE_UTF8. The JS API does not
       currently support any other encoding and likely never
       will. This function does not replace that argument on its own
       because it may contain other flags. As a special case, if
       the bottom 4 bits of that argument are 0, SQLITE_UTF8 is
       assumed.

       2) Any of the four final arguments may be either WASM pointers
       (assumed to be function pointers) or JS Functions. In the
       latter case, each gets bound to WASM using
       sqlite3.capi.wasm.installFunction() and that wrapper is passed
       on to the native implementation.

       For consistency with the C API, it requires the same number of
       arguments. It returns capi.SQLITE_MISUSE if passed any other
       argument count.

       The semantics of JS functions are:

       xFunc: is passed `(pCtx, ...values)`. Its return value becomes
       the new SQL function's result.

       xStep: is passed `(pCtx, ...values)`. Its return value is
       ignored.

       xFinal: is passed `(pCtx)`. Its return value becomes the new
       aggregate SQL function's result.

       xDestroy: is passed `(void*)`. Its return value is ignored. The
       pointer passed to it is the one from the 5th argument to
       sqlite3_create_function_v2().

       Note that:

       - `pCtx` in the above descriptions is a `sqlite3_context*`. At
         least 99 times out of a hundred, that initial argument will
         be irrelevant for JS UDF bindings, but it needs to be there
         so that the cases where it _is_ relevant, in particular with
         window and aggregate functions, have full access to the
         lower-level sqlite3 APIs.

       - When wrapping JS functions, the remaining arguments are passd
         to them as positional arguments, not as an array of
         arguments, because that allows callback definitions to be
         more JS-idiomatic than C-like. For example `(pCtx,a,b)=>a+b`
         is more intuitive and legible than
         `(pCtx,args)=>args[0]+args[1]`. For cases where an array of
         arguments would be more convenient, the callbacks simply need
         to be declared like `(pCtx,...args)=>{...}`, in which case
         `args` will be an array.

       - If a JS wrapper throws, it gets translated to
         sqlite3_result_error() or sqlite3_result_error_nomem(),
         depending on whether the exception is an
         sqlite3.WasmAllocError object or not.

       - When passing on WASM function pointers, arguments are _not_
         converted or reformulated. They are passed on as-is in raw
         pointer form using their native C signatures. Only JS
         functions passed in to this routine, and thus wrapped by this
         routine, get automatic conversions of arguments and result
         values. The routines which perform those conversions are
         exposed for client-side use as sqlite3_values_to_js(),
         sqlite3_result_js(), and sqlite3_result_error_js().

       For xFunc(), xStep(), and xFinal():

       - When called from SQL, arguments to the UDF, and its result,
         will be converted between JS and SQL with as much fidelity as
         is feasible, triggering an exception if a type conversion
         cannot be determined. Some freedom is afforded to numeric
         conversions due to friction between the JS and C worlds:
         integers which are larger than 32 bits may be treated as
         doubles or BigInts.

       If any JS-side bound functions throw, those exceptions are
       intercepted and converted to database-side errors with the
       exception of xDestroy(): any exception from it is ignored,
       possibly generating a console.error() message.  Destructors
       must not throw.

       Automatically-converted JS-to-WASM functions will be cleaned up
       either when (A) this function is called again with the same
       name, arity, and encoding, but null/0 values for the functions,
       or (B) when pDb is passed to sqlite3_close_v2(). If this factor
       is relevant for a given client, they can create WASM-bound JS
       functions themselves, hold on to their pointers, and pass the
       pointers in to here. Later on, they can free those pointers
       (using `wasm.uninstallFunction()` or equivalent).

       C reference: https://sqlite.org/c3ref/create_function.html

       Maintenance reminder: the ability to add new
       WASM-accessible functions to the runtime requires that the
       WASM build is compiled with emcc's `-sALLOW_TABLE_GROWTH`
       flag.
    */
    sqlite3_create_function_v2: (
      pDb, funcName, nArg, eTextRep, pApp,
      xFunc, xStep, xFinal, xDestroy
    )=>{/*installed later*/},
    /**
       Equivalent to passing the same arguments to
       sqlite3_create_function_v2(), with 0 as the final argument.
    */
    sqlite3_create_function: (
      pDb, funcName, nArg, eTextRep, pApp,
      xFunc, xStep, xFinal
    )=>{/*installed later*/},
    /**
       The sqlite3_create_window_function() JS wrapper differs from
       its native implementation in the exact same way that
       sqlite3_create_function_v2() does. The additional function,
       xInverse(), is treated identically to xStep() by the wrapping
       layer.
    */
    sqlite3_create_window_function: (
      pDb, funcName, nArg, eTextRep, pApp,
      xStep, xFinal, xValue, xInverse, xDestroy
    )=>{/*installed later*/},
    /**
       The sqlite3_prepare_v3() binding handles two different uses
       with differing JS/WASM semantics:

       1) sqlite3_prepare_v3(pDb, sqlString, -1, prepFlags, ppStmt , null)

       2) sqlite3_prepare_v3(pDb, sqlPointer, sqlByteLen, prepFlags, ppStmt, sqlPointerToPointer)

       The SQL length argument (the 3rd argument) must, for usage (1),
       always be negative because it must be a byte length and that
       value is expensive to calculate from JS (where only the
       character length of strings is readily available). It is
       retained in this API's interface for code/documentation
       compatibility reasons but is currently _always_ ignored. With
       usage (2), the 3rd argument is used as-is but is is still
       critical that the C-style input string (2nd argument) be
       terminated with a 0 byte.

       In usage (1), the 2nd argument must be of type string,
       Uint8Array, Int8Array, or ArrayBuffer (all of which are assumed
       to hold SQL). If it is, this function assumes case (1) and
       calls the underyling C function with the equivalent of:

       (pDb, sqlAsString, -1, prepFlags, ppStmt, null)

       The `pzTail` argument is ignored in this case because its
       result is meaningless when a string-type value is passed
       through: the string goes through another level of internal
       conversion for WASM's sake and the result pointer would refer
       to that transient conversion's memory, not the passed-in
       string.

       If the sql argument is not a string, it must be a _pointer_ to
       a NUL-terminated string which was allocated in the WASM memory
       (e.g. using capi.wasm.alloc() or equivalent). In that case,
       the final argument may be 0/null/undefined or must be a pointer
       to which the "tail" of the compiled SQL is written, as
       documented for the C-side sqlite3_prepare_v3().

       In case (2), the underlying C function is called with the
       equivalent of:

       (pDb, sqlAsPointer, sqlByteLen, prepFlags, ppStmt, pzTail)

       It returns its result and compiled statement as documented in
       the C API. Fetching the output pointers (5th and 6th
       parameters) requires using `capi.wasm.peek()` (or
       equivalent) and the `pzTail` will point to an address relative to
       the `sqlAsPointer` value.

       If passed an invalid 2nd argument type, this function will
       return SQLITE_MISUSE and sqlite3_errmsg() will contain a string
       describing the problem.

       Side-note: if given an empty string, or one which contains only
       comments or an empty SQL expression, 0 is returned but the result
       output pointer will be NULL.
    */
    sqlite3_prepare_v3: (dbPtr, sql, sqlByteLen, prepFlags,
                         stmtPtrPtr, strPtrPtr)=>{}/*installed later*/,

    /**
       Equivalent to calling sqlite3_prapare_v3() with 0 as its 4th argument.
    */
    sqlite3_prepare_v2: (dbPtr, sql, sqlByteLen,
                         stmtPtrPtr,strPtrPtr)=>{}/*installed later*/,

    /**
       This binding enables the callback argument to be a JavaScript.

       If the callback is a function, then for the duration of the
       sqlite3_exec() call, it installs a WASM-bound function which
       acts as a proxy for the given callback. That proxy will also
       perform a conversion of the callback's arguments from
       `(char**)` to JS arrays of strings. However, for API
       consistency's sake it will still honor the C-level callback
       parameter order and will call it like:

       `callback(pVoid, colCount, listOfValues, listOfColNames)`

       If the callback is not a JS function then this binding performs
       no translation of the callback, but the sql argument is still
       converted to a WASM string for the call using the
       "string:flexible" argument converter.
    */
    sqlite3_exec: (pDb, sql, callback, pVoid, pErrMsg)=>{}/*installed later*/,

    /**
       If passed a single argument which appears to be a byte-oriented
       TypedArray (Int8Array or Uint8Array), this function treats that
       TypedArray as an output target, fetches `theArray.byteLength`
       bytes of randomness, and populates the whole array with it. As
       a special case, if the array's length is 0, this function
       behaves as if it were passed (0,0). When called this way, it
       returns its argument, else it returns the `undefined` value.

       If called with any other arguments, they are passed on as-is
       to the C API. Results are undefined if passed any incompatible
       values.
     */
    sqlite3_randomness: (n, outPtr)=>{/*installed later*/},
  }/*capi*/);

  /**
     Various internal-use utilities are added here as needed. They
     are bound to an object only so that we have access to them in
     the differently-scoped steps of the API bootstrapping
     process. At the end of the API setup process, this object gets
     removed. These are NOT part of the public API.
  */
  const util = {
    affirmBindableTypedArray, flexibleString,
    bigIntFits32, bigIntFits64, bigIntFitsDouble,
    isBindableTypedArray,
    isInt32, isSQLableTypedArray, isTypedArray,
    isUIThread: ()=>(globalThis.window===globalThis && !!globalThis.document),
    // is this true for ESM?: 'undefined'===typeof WorkerGlobalScope
    toss: function(...args){throw new Error(args.join(' '))},
    toss3,
    typedArrayPart: wasm.typedArrayPart,
    nu,
    assert: function(arg,msg){
      if( !arg ){
        util.toss("Assertion failed:",msg);
      }
    },
    /**
       Given a byte array or ArrayBuffer, this function throws if the
       lead bytes of that buffer do not hold a SQLite3 database header,
       else it returns without side effects.

       Added in 3.44.
    */
    affirmDbHeader: function(bytes){
      if(bytes instanceof ArrayBuffer) bytes = new Uint8Array(bytes);
      const header = "SQLite format 3";
      if( header.length > bytes.byteLength ){
        toss3("Input does not contain an SQLite3 database header.");
      }
      for(let i = 0; i < header.length; ++i){
        if( header.charCodeAt(i) !== bytes[i] ){
          toss3("Input does not contain an SQLite3 database header.");
        }
      }
    },
    /**
       Given a byte array or ArrayBuffer, this function throws if the
       database does not, at a cursory glance, appear to be an SQLite3
       database. It only examines the size and header, but further
       checks may be added in the future.

       Added in 3.44.
    */
    affirmIsDb: function(bytes){
      if(bytes instanceof ArrayBuffer) bytes = new Uint8Array(bytes);
      const n = bytes.byteLength;
      if(n<512 || n%512!==0) {
        toss3("Byte array size",n,"is invalid for an SQLite3 db.");
      }
      util.affirmDbHeader(bytes);
    }
  }/*util*/;

  /**
     wasm.X properties which are used for configuring the wasm
     environment via whwashutil.js. This object gets fleshed out with
     a number of WASM-specific utilities, in sqlite3-api-glue.c-pp.js.
  */
  Object.assign(wasm, {

    /**
       The symbols exported by the WASM environment.
    */
    exports: config.exports
      || toss3("Missing API config.exports (WASM module exports)."),

    /**
       When Emscripten compiles with `-sIMPORTED_MEMORY`, it
       initializes the heap and imports it into wasm, as opposed to
       the other way around. In this case, the memory is not available
       via this.exports.memory so the client must pass it in via
       config.memory.
    */
    memory: config.memory
      || config.exports['memory']
      || toss3("API config object requires a WebAssembly.Memory object",
              "in either config.exports.memory (exported)",
              "or config.memory (imported)."),

    /**
       The WASM pointer size. If set then it MUST be one of 4 or 8 and
       it MUST correspond to the WASM environment's pointer size. We
       figure out the size by calling some un-JS-wrapped WASM function
       which returns a pointer-type value. If that value is a BigInt,
       it's 64-bit, else it's 32-bit. The pieces which populate
       sqlite3.wasm (whwasmutil.js) can figure this out _if_ they can
       allocate, but we have a chicken/egg situation there which makes
       it illegal for that code to invoke wasm.dealloc() at the time
       it would be needed. So we need to configure it ahead of time
       (here) instead.
    */
    pointerSize: ('number'===typeof config.exports.sqlite3_libversion()) ? 4 : 8,

    /**
       True if BigInt support was enabled via (e.g.) the
       Emscripten -sWASM_BIGINT flag, else false. When
       enabled, certain 64-bit sqlite3 APIs are enabled which
       are not otherwise enabled due to JS/WASM int64
       impedance mismatches.
    */
    bigIntEnabled: !!config.bigIntEnabled,

    /**
       WebAssembly.Table object holding the indirect function call
       table. Defaults to exports.__indirect_function_table.
    */
    functionTable: config.functionTable,

    /**
       The API's primary point of access to the WASM-side memory
       allocator.  Works like sqlite3_malloc() but throws a
       WasmAllocError if allocation fails. It is important that any
       code which might pass through the sqlite3 C API NOT throw and
       must instead return SQLITE_NOMEM (or equivalent, depending on
       the context).

       Very few cases in the sqlite3 JS APIs can result in
       client-defined functions propagating exceptions via the C-style
       API. Most notably, this applies to WASM-bound JS functions
       which are created directly by clients and passed on _as WASM
       function pointers_ to functions such as
       sqlite3_create_function_v2(). Such bindings created
       transparently by this API will automatically use wrappers which
       catch exceptions and convert them to appropriate error codes.

       For cases where non-throwing allocation is required, use
       this.alloc.impl(), which is the unadulterated WASM-exported
       counterpart of this wrapper.

       Design note: this function is not named "malloc" primarily
       because Emscripten uses that name and we wanted to avoid any
       confusion early on in this code's development, when it still
       had close ties to Emscripten's glue code.
    */
    alloc: undefined/*installed later*/,

    /**
       Rarely necessary in JS code, this routine works like
       sqlite3_realloc(M,N), where M is either NULL or a pointer
       obtained from this function or this.alloc() and N is the number
       of bytes to reallocate the block to. Returns a pointer to the
       reallocated block or 0 if allocation fails.

       If M is NULL and N is positive, this behaves like
       this.alloc(N). If N is 0, it behaves like this.dealloc().
       Results are undefined if N is negative (sqlite3_realloc()
       treats that as 0, but if this code is built with a different
       allocator it may misbehave with negative values).

       Like this.alloc.impl(), this.realloc.impl() is a direct binding
       to the underlying realloc() implementation which does not throw
       exceptions, instead returning 0 (or 0n) on allocation error.
    */
    realloc: undefined/*installed later*/,

    /**
       The API's primary point of access to the WASM-side memory
       deallocator. Works like sqlite3_free().

       Design note: this function is not named "free" for the same
       reason that this.alloc() is not called this.malloc().
    */
    dealloc: undefined/*installed later*/

    /* Many more wasm-related APIs get installed later on. */
  }/*wasm*/);

  /**
     wasm.alloc()'s srcTypedArray.byteLength bytes,
     populates them with the values from the source
     TypedArray, and returns the pointer to that memory. The
     returned pointer must eventually be passed to
     wasm.dealloc() to clean it up.

     The argument may be a Uint8Array, Int8Array, or ArrayBuffer,
     and it throws if passed any other type.

     As a special case, to avoid further special cases where
     this is used, if srcTypedArray.byteLength is 0, it
     allocates a single byte and sets it to the value
     0. Even in such cases, calls must behave as if the
     allocated memory has exactly srcTypedArray.byteLength
     bytes.
  */
  wasm.allocFromTypedArray = function(srcTypedArray){
    if(srcTypedArray instanceof ArrayBuffer){
      srcTypedArray = new Uint8Array(srcTypedArray);
    }
    affirmBindableTypedArray(srcTypedArray);
    const pRet = wasm.alloc(srcTypedArray.byteLength || 1);
    wasm.heapForSize(srcTypedArray.constructor)
      .set(srcTypedArray.byteLength ? srcTypedArray : [0], Number(pRet))
    /* Maintenance note: the order of alloc() and heapForSize() calls
       is significant: https://sqlite.org/forum/forumpost/05b77273be104532 */;
    return pRet;
  };

  {
    // Set up allocators...
    const keyAlloc = config.allocExportName,
          keyDealloc = config.deallocExportName,
          keyRealloc = config.reallocExportName;
    for(const key of [keyAlloc, keyDealloc, keyRealloc]){
      const f = wasm.exports[key];
      if(!(f instanceof Function)) toss3("Missing required exports[",key,"] function.");
    }

    wasm.alloc = function f(n){
      return f.impl(n) || WasmAllocError.toss("Failed to allocate",n," bytes.");
    };
    wasm.alloc.impl = wasm.exports[keyAlloc];
    wasm.realloc = function f(m,n){
      const m2 = f.impl(wasm.ptr.coerce(m)/*tag:64bit*/,n);
      return n ? (m2 || WasmAllocError.toss("Failed to reallocate",n," bytes.")) : wasm.ptr.null;
    };
    wasm.realloc.impl = wasm.exports[keyRealloc];
    wasm.dealloc = function f(m){
      f.impl(wasm.ptr.coerce(m)/*tag:64bit*/)
      /* This coerce() is the reason we have to set wasm.pointerSize before
         calling WhWasmUtilInstaller(). If we don't, that code will call
         into this very early in its init, before wasm.ptr has been set up,
         resulting in a null deref here. */;
    };
    wasm.dealloc.impl = wasm.exports[keyDealloc];
  }

  /**
     Reports info about compile-time options using
     sqlite3_compileoption_get() and sqlite3_compileoption_used(). It
     has several distinct uses:

     If optName is an array then it is expected to be a list of
     compilation options and this function returns an object
     which maps each such option to true or false, indicating
     whether or not the given option was included in this
     build. That object is returned.

     If optName is an object, its keys are expected to be compilation
     options and this function sets each entry to true or false,
     indicating whether the compilation option was used or not. That
     object is returned.

     If passed no arguments then it returns an object mapping
     all known compilation options to their compile-time values,
     or boolean true if they are defined with no value. This
     result, which is relatively expensive to compute, is cached
     and returned for future no-argument calls.

     In all other cases it returns true if the given option was
     active when when compiling the sqlite3 module, else false.

     Compile-time option names may optionally include their
     "SQLITE_" prefix. When it returns an object of all options,
     the prefix is elided.
  */
  wasm.compileOptionUsed = function f(optName){
    if(!arguments.length){
      if(f._result) return f._result;
      else if(!f._opt){
        f._rx = /^([^=]+)=(.+)/;
        f._rxInt = /^-?\d+$/;
        f._opt = function(opt, rv){
          const m = f._rx.exec(opt);
          rv[0] = (m ? m[1] : opt);
          rv[1] = m ? (f._rxInt.test(m[2]) ? +m[2] : m[2]) : true;
        };
      }
      const rc = nu(), ov = [0,0];
      let i = 0, k;
      while((k = capi.sqlite3_compileoption_get(i++))){
        f._opt(k,ov);
        rc[ov[0]] = ov[1];
      }
      return f._result = rc;
    }else if(Array.isArray(optName)){
      const rc = nu();
      optName.forEach((v)=>{
        rc[v] = capi.sqlite3_compileoption_used(v);
      });
      return rc;
    }else if('object' === typeof optName){
      Object.keys(optName).forEach((k)=> {
        optName[k] = capi.sqlite3_compileoption_used(k);
      });
      return optName;
    }
    return (
      'string'===typeof optName
    ) ? !!capi.sqlite3_compileoption_used(optName) : false;
  }/*compileOptionUsed()*/;

  /**
     sqlite3.wasm.pstack (pseudo-stack) holds a special-case allocator
     intended solely for short-lived, small data. In practice, it's
     primarily used to allocate output pointers. It must not be used
     for any memory which needs to outlive the scope in which it's
     obtained from pstack.

     The library guarantees only that a minimum of 2kb are available
     in this allocator, and it may provide more (it's a build-time
     value). pstack.quota and pstack.remaining can be used to get the
     total resp. remaining amount of memory.

     It has only a single intended usage pattern:

     ```
     const stackPos = pstack.pointer;
     try{
       const ptr = pstack.alloc(8);
       // ==> pstack.pointer === ptr
       const otherPtr = pstack.alloc(8);
       // ==> pstack.pointer === otherPtr
       ...
     }finally{
       pstack.restore(stackPos);
       // ==> pstack.pointer === stackPos
     }
     ```

     This allocator is much faster than a general-purpose one but is
     limited to usage patterns like the one shown above (which are
     pretty common when using sqlite3.capi).

     The memory lives in the WASM heap and can be used with routines
     such as wasm.poke() and wasm.heap8u().slice().
  */
  wasm.pstack = nu({
    /**
       Sets the current pstack position to the given pointer. Results
       are undefined if the passed-in value did not come from
       this.pointer.

       In debug builds this may trigger an assert() in the WASM
       environment if passed an illegal value.
    */
    restore: wasm.exports.sqlite3__wasm_pstack_restore,

    /**
       Attempts to allocate the given number of bytes from the
       pstack. On success, it zeroes out a block of memory of the
       given size, adjusts the pstack pointer, and returns a pointer
       to the memory. On error, throws a WasmAllocError. The
       memory must eventually be released using restore().

       If n is a string, it must be a WASM "IR" value in the set
       accepted by wasm.sizeofIR(), which is mapped to the size of
       that data type. If passed a string not in that set, it throws a
       WasmAllocError.

       This method always adjusts the given value to be a multiple
       of 8 bytes because failing to do so can lead to incorrect
       results when reading and writing 64-bit values from/to the WASM
       heap. Similarly, the returned address is always 8-byte aligned.
    */
    alloc: function(n){
      if('string'===typeof n && !(n = wasm.sizeofIR(n))){
        WasmAllocError.toss("Invalid value for pstack.alloc(",arguments[0],")");
      }
      return wasm.exports.sqlite3__wasm_pstack_alloc(n)
        || WasmAllocError.toss("Could not allocate",n,
                               "bytes from the pstack.");
    },

    /**
       alloc()'s n chunks, each sz bytes, as a single memory block and
       returns the addresses as an array of n element, each holding
       the address of one chunk.

       sz may optionally be an IR string accepted by wasm.sizeofIR().

       Throws a WasmAllocError if allocation fails.

       Example:

       ```
       const [p1, p2, p3] = wasm.pstack.allocChunks(3, wasm.ptr.size);
       ```
    */
    allocChunks: function(n,sz){
      if('string'===typeof sz && !(sz = wasm.sizeofIR(sz))){
        WasmAllocError.toss("Invalid size value for allocChunks(",arguments[1],")");
      }
      const mem = wasm.pstack.alloc(n * sz);
      const rc = [mem];
      let i = 1, offset = sz;
      for(; i < n; ++i, offset += sz) rc.push(wasm.ptr.add(mem, offset));
      return rc;
    },

    /**
       A convenience wrapper for allocChunks() which sizes each chunk
       as either 8 bytes (safePtrSize is truthy) or wasm.ptr.size (if
       safePtrSize is falsy).

       How it returns its result differs depending on its first
       argument: if it's 1, it returns a single pointer value. If it's
       more than 1, it returns the same as allocChunks().

       When a returned pointer will refer to a 64-bit value, e.g. a
       double or int64, and that value must be written or fetched,
       e.g. using wasm.poke() or wasm.peek(), it is
       important that the pointer in question be aligned to an 8-byte
       boundary or else it will not be fetched or written properly and
       will corrupt or read neighboring memory.

       However, when all pointers involved point to "small" data, it
       is safe to pass a falsy value to save a tiny bit of memory.
    */
    allocPtr: (n=1,safePtrSize=true)=>{
      return 1===n
        ? wasm.pstack.alloc(safePtrSize ? 8 : wasm.ptr.size)
        : wasm.pstack.allocChunks(n, safePtrSize ? 8 : wasm.ptr.size);
    },

    /**
       Records the current pstack position, calls the given function,
       passing it the sqlite3 object, then restores the pstack
       regardless of whether the function throws. Returns the result
       of the call or propagates an exception on error.

       Added in 3.44.
    */
    call: function(f){
      const stackPos = wasm.pstack.pointer;
      try{ return f(sqlite3) }
      finally{ wasm.pstack.restore(stackPos); }
    }

  })/*wasm.pstack*/;

  Object.defineProperties(wasm.pstack, {
    /**
       Resolves to the current pstack position pointer either as a
       Number (32-bit WASM) or BigInt (64-bit WASM). This value is
       intended _only_ to be saved for passing to restore(). Writing
       to this memory, without first reserving it via
       wasm.pstack.alloc() and friends, leads to undefined results.
    */
    pointer: {
      configurable: false, iterable: true, writeable: false,
      get: wasm.exports.sqlite3__wasm_pstack_ptr
      //Whether or not a setter as an alternative to restore() is
      //clearer or would just lead to confusion or misuse is unclear.
      //set: wasm.exports.sqlite3__wasm_pstack_restore
    },

    /**
       Resolves to the total number of bytes available in the pstack
       allocator, including any space which is currently
       allocated. This value is a compile-time constant.
    */
    quota: {
      configurable: false, iterable: true, writeable: false,
      get: wasm.exports.sqlite3__wasm_pstack_quota
    },

    /**
       Resolves to the number of bytes remaining in the pstack
       allocator.
    */
    remaining: {
      configurable: false, iterable: true, writeable: false,
      get: wasm.exports.sqlite3__wasm_pstack_remaining
    }
  })/*wasm.pstack properties*/;

  /**
     Docs: https://sqlite.org/wasm/doc/trunk/api-c-style.md#sqlite3_randomness
  */
  capi.sqlite3_randomness = (...args)=>{
    if(1===args.length
       && util.isTypedArray(args[0])
       && 1===args[0].BYTES_PER_ELEMENT){
      const ta = args[0];
      if(0===ta.byteLength){
        wasm.exports.sqlite3_randomness(0,wasm.ptr.null);
        return ta;
      }
      const stack = wasm.pstack.pointer;
      try {
        let n = ta.byteLength, offset = 0;
        const r = wasm.exports.sqlite3_randomness;
        const heap = wasm.heap8u();
        const nAlloc = n < 512 ? n : 512;
        const ptr = wasm.pstack.alloc(nAlloc);
        do{
          const j = (n>nAlloc ? nAlloc : n);
          r(j, ptr);
          ta.set(wasm.typedArrayPart(heap, ptr, wasm.ptr.add(ptr,j)), offset);
          n -= j;
          offset += j;
        } while(n > 0);
      }catch(e){
        config.error("Highly unexpected (and ignored!) "+
                     "exception in sqlite3_randomness():",e);
      }finally{
        wasm.pstack.restore(stack);
      }
      return ta;
    }
    wasm.exports.sqlite3_randomness(...args);
  };

  /**
     If the wasm environment has a WASMFS/OPFS-backed persistent
     storage directory, its path is returned by this function. If it
     does not then it returns "" (noting that "" is a falsy value).

     The first time this is called, this function inspects the current
     environment to determine whether WASMFS persistence support is
     available and, if it is, enables it (if needed). After the first
     call it always returns the cached result.

     If the returned string is not empty, any files stored under the
     returned path (recursively) are housed in OPFS storage. If the
     returned string is empty, this particular persistent storage
     option is not available on the client.

     Though the mount point name returned by this function is intended
     to remain stable, clients should not hard-coded it anywhere.
     Always call this function to get the path.

     This function is a no-op in most builds of this library, as the
     WASMFS capability requires a custom build.
  */
  capi.sqlite3_wasmfs_opfs_dir = function(){
    if(undefined !== this.dir) return this.dir;
    // If we have no OPFS, there is no persistent dir
    const pdir = config.wasmfsOpfsDir;
    if(!pdir
       || !globalThis.FileSystemHandle
       || !globalThis.FileSystemDirectoryHandle
       || !globalThis.FileSystemFileHandle
       || !wasm.exports.sqlite3__wasm_init_wasmfs){
      return this.dir = "";
    }
    try{
      if(pdir && 0===wasm.xCallWrapped(
        'sqlite3__wasm_init_wasmfs', 'i32', ['string'], pdir
      )){
        return this.dir = pdir;
      }else{
        return this.dir = "";
      }
    }catch(e){
      // sqlite3__wasm_init_wasmfs() is not available
      return this.dir = "";
    }
  }.bind(nu());

  /**
     Returns true if sqlite3.capi.sqlite3_wasmfs_opfs_dir() is a
     non-empty string and the given name starts with (that string +
     '/'), else returns false.
  */
  capi.sqlite3_wasmfs_filename_is_persistent = function(name){
    const p = capi.sqlite3_wasmfs_opfs_dir();
    return (p && name) ? name.startsWith(p+'/') : false;
  };

  /**
     Given an `sqlite3*`, an sqlite3_vfs name, and an optional db name
     (defaulting to "main"), returns a truthy value (see below) if
     that db uses that VFS, else returns false. If pDb is falsy then
     the 3rd argument is ignored and this function returns a truthy
     value if the default VFS name matches that of the 2nd argument.
     Results are undefined if pDb is truthy but refers to an invalid
     pointer. The 3rd argument specifies the database name of the
     given database connection to check, defaulting to the main db.

     The 2nd and 3rd arguments may either be a JS string or a WASM
     C-string. If the 2nd argument is a NULL WASM pointer, the default
     VFS is assumed. If the 3rd is a NULL WASM pointer, "main" is
     assumed.

     The truthy value it returns is a pointer to the `sqlite3_vfs`
     object.

     To permit safe use of this function from APIs which may be called
     via C (like SQL UDFs), this function does not throw: if bad
     arguments cause a conversion error when passing into wasm-space,
     false is returned.
  */
  capi.sqlite3_js_db_uses_vfs = function(pDb,vfsName,dbName=0){
    try{
      const pK = capi.sqlite3_vfs_find(vfsName);
      if(!pK) return false;
      else if(!pDb){
        return pK===capi.sqlite3_vfs_find(0) ? pK : false;
      }else{
        return pK===capi.sqlite3_js_db_vfs(pDb,dbName) ? pK : false;
      }
    }catch(e){
      /* Ignore - probably bad args to a wasm-bound function. */
      return false;
    }
  };

  /**
     Returns an array of the names of all currently-registered sqlite3
     VFSes.
  */
  capi.sqlite3_js_vfs_list = function(){
    const rc = [];
    let pVfs = capi.sqlite3_vfs_find(wasm.ptr.null);
    while(pVfs){
      const oVfs = new capi.sqlite3_vfs(pVfs);
      rc.push(wasm.cstrToJs(oVfs.$zName));
      pVfs = oVfs.$pNext;
      oVfs.dispose();
    }
    return rc;
  };

  /**
     A convenience wrapper around sqlite3_serialize() which serializes
     the given `sqlite3*` pointer to a Uint8Array. The first argument
     may be either an `sqlite3*` or an sqlite3.oo1.DB instance.

     On success it returns a Uint8Array. If the schema is empty, an
     empty array is returned.

     `schema` is the schema to serialize. It may be a WASM C-string
     pointer or a JS string. If it is falsy, it defaults to `"main"`.

     On error it throws with a description of the problem.
  */
  capi.sqlite3_js_db_export = function(pDb, schema=0){
    pDb = wasm.xWrap.testConvertArg('sqlite3*', pDb);
    if(!pDb) toss3('Invalid sqlite3* argument.');
    if(!wasm.bigIntEnabled) toss3('BigInt support is not enabled.');
    const scope = wasm.scopedAllocPush();
    let pOut;
    try{
      const pSize = wasm.scopedAlloc(8/*i64*/ + wasm.ptr.size);
      const ppOut = wasm.ptr.add(pSize, 8);
      /**
         Maintenance reminder, since this cost a full hour of grief
         and confusion: if the order of pSize/ppOut are reversed in
         that memory block, fetching the value of pSize after the
         export reads a garbage size because it's not on an 8-byte
         memory boundary!
      */
      const zSchema = schema
            ? (wasm.isPtr(schema) ? schema : wasm.scopedAllocCString(''+schema))
            : wasm.ptr.null;
      let rc = wasm.exports.sqlite3__wasm_db_serialize(
        pDb, zSchema, ppOut, pSize, 0
      );
      if(rc){
        toss3("Database serialization failed with code",
             sqlite3.capi.sqlite3_js_rc_str(rc));
      }
      pOut = wasm.peekPtr(ppOut);
      const nOut = wasm.peek(pSize, 'i64');
      rc = nOut
        ? wasm.heap8u().slice(Number(pOut), Number(pOut) + Number(nOut))
        : new Uint8Array();
      return rc;
    }finally{
      if(pOut) wasm.exports.sqlite3_free(pOut);
      wasm.scopedAllocPop(scope);
    }
  };

  /**
     Given a `sqlite3*` and a database name (JS string or WASM
     C-string pointer, which may be 0), returns a pointer to the
     sqlite3_vfs responsible for it. If the given db name is null/0,
     or not provided, then "main" is assumed.
  */
  capi.sqlite3_js_db_vfs =
    (dbPointer, dbName=wasm.ptr.null)=>util.sqlite3__wasm_db_vfs(dbPointer, dbName);

  /**
     A thin wrapper around capi.sqlite3_aggregate_context() which
     behaves the same except that it throws a WasmAllocError if that
     function returns 0. As a special case, if n is falsy it does
     _not_ throw if that function returns 0. That special case is
     intended for use with xFinal() implementations.
  */
  capi.sqlite3_js_aggregate_context = (pCtx, n)=>{
    return capi.sqlite3_aggregate_context(pCtx, n)
      || (n ? WasmAllocError.toss("Cannot allocate",n,
                                  "bytes for sqlite3_aggregate_context()")
          : 0);
  };

  /**
     If the current environment supports the POSIX file APIs, this routine
     creates (or overwrites) the given file using those APIs. This is
     primarily intended for use in Emscripten-based builds where the POSIX
     APIs are transparently proxied by an in-memory virtual filesystem.
     It may behave differently in other environments.

     The first argument must be either a JS string or WASM C-string
     holding the filename. This routine does _not_ create intermediary
     directories if the filename has a directory part.

     The 2nd argument may either a valid WASM memory pointer, an
     ArrayBuffer, or a Uint8Array. The 3rd must be the length, in
     bytes, of the data array to copy. If the 2nd argument is an
     ArrayBuffer or Uint8Array and the 3rd is not a positive integer
     then the 3rd defaults to the array's byteLength value.

     Results are undefined if data is a WASM pointer and dataLen is
     exceeds data's bounds.

     Throws if any arguments are invalid or if creating or writing to
     the file fails.

     Added in 3.43 as an alternative for the deprecated
     sqlite3_js_vfs_create_file().
  */
  capi.sqlite3_js_posix_create_file = function(filename, data, dataLen){
    let pData;
    if(data && wasm.isPtr(data)){
      pData = data;
    }else if(data instanceof ArrayBuffer || data instanceof Uint8Array){
      pData = wasm.allocFromTypedArray(data);
      if(arguments.length<3 || !util.isInt32(dataLen) || dataLen<0){
        dataLen = data.byteLength;
      }
    }else{
      SQLite3Error.toss("Invalid 2nd argument for sqlite3_js_posix_create_file().");
    }
    try{
      if(!util.isInt32(dataLen) || dataLen<0){
        SQLite3Error.toss("Invalid 3rd argument for sqlite3_js_posix_create_file().");
      }
      const rc = util.sqlite3__wasm_posix_create_file(filename, pData, dataLen);
      if(rc) SQLite3Error.toss("Creation of file failed with sqlite3 result code",
                               capi.sqlite3_js_rc_str(rc));
    }finally{
      if( pData && pData!==data ) wasm.dealloc(pData);
    }
  };

  /**
     Deprecation warning: this function does not work properly in
     debug builds of sqlite3 because its out-of-scope use of the
     sqlite3_vfs API triggers assertions in the core library.  That
     was unfortunately not discovered until 2023-08-11. This function
     is now deprecated. It should not be used in new code and should
     be removed from existing code.

     Alternative options:

     - The "unix" VFS and its variants can get equivalent
       functionality with sqlite3_js_posix_create_file().

     - OPFS: use either sqlite3.oo1.OpfsDb.importDb(), for the "opfs"
       VFS, or the importDb() method of the PoolUtil object provided
       by the "opfs-sahpool" OPFS (noting that its VFS name may differ
       depending on client-side configuration). We cannot proxy those
       from here because the former is necessarily asynchronous and
       the latter requires information not available to this function.

     Historical (deprecated) behaviour:

     Creates a file using the storage appropriate for the given
     sqlite3_vfs.  The first argument may be a VFS name (JS string
     only, NOT a WASM C-string), WASM-managed `sqlite3_vfs*`, or
     a capi.sqlite3_vfs instance. Pass 0 (a NULL pointer) to use the
     default VFS. If passed a string which does not resolve using
     sqlite3_vfs_find(), an exception is thrown. (Note that a WASM
     C-string is not accepted because it is impossible to
     distinguish from a C-level `sqlite3_vfs*`.)

     The second argument, the filename, must be a JS or WASM C-string.

     The 3rd may either be falsy, a valid WASM memory pointer, an
     ArrayBuffer, or a Uint8Array. The 4th must be the length, in
     bytes, of the data array to copy. If the 3rd argument is an
     ArrayBuffer or Uint8Array and the 4th is not a positive integer
     then the 4th defaults to the array's byteLength value.

     If data is falsy then a file is created with dataLen bytes filled
     with uninitialized data (whatever truncate() leaves there). If
     data is not falsy then a file is created or truncated and it is
     filled with the first dataLen bytes of the data source.

     Throws if any arguments are invalid or if creating or writing to
     the file fails.

     Note that most VFSes do _not_ automatically create directory
     parts of filenames, nor do all VFSes have a concept of
     directories.  If the given filename is not valid for the given
     VFS, an exception will be thrown. This function exists primarily
     to assist in implementing file-upload capability, with the caveat
     that clients must have some idea of the VFS into which they want
     to upload and that VFS must support the operation.

     VFS-specific notes:

     - "memdb": results are undefined.

     - "kvvfs": will fail with an I/O error due to strict internal
       requirements of that VFS's xTruncate().

     - "unix" and related: will use the WASM build's equivalent of the
       POSIX I/O APIs. This will work so long as neither a specific
       VFS nor the WASM environment imposes requirements which break
       it.  (Much later: it turns out that debug builds of the library
       impose such requirements, in that they assert() that dataLen is
       an even multiple of a valid db page size.)

     - "opfs": uses OPFS storage and creates directory parts of the
       filename. It can only be used to import an SQLite3 database
       file and will fail if given anything else.
  */
  capi.sqlite3_js_vfs_create_file = function(vfs, filename, data, dataLen){
    config.warn("sqlite3_js_vfs_create_file() is deprecated and",
                "should be avoided because it can lead to C-level crashes.",
                "See its documentation for alternatives.");
    let pData;
    if(data){
      if( wasm.isPtr(data) ){
        pData = data;
      }else{
        if( data instanceof ArrayBuffer ){
          data = new Uint8Array(data);
        }
        if( data instanceof Uint8Array ){
          pData = wasm.allocFromTypedArray(data);
          if(arguments.length<4 || !util.isInt32(dataLen) || dataLen<0){
            dataLen = data.byteLength;
          }
        }else{
          SQLite3Error.toss("Invalid 3rd argument type for sqlite3_js_vfs_create_file().");
        }
      }
    }else{
       pData = 0;
    }
    if(!util.isInt32(dataLen) || dataLen<0){
      if( pData && pData!==data ) wasm.dealloc(pData);
      SQLite3Error.toss("Invalid 4th argument for sqlite3_js_vfs_create_file().");
    }
    try{
      const rc = util.sqlite3__wasm_vfs_create_file(vfs, filename, pData, dataLen);
      if(rc) SQLite3Error.toss("Creation of file failed with sqlite3 result code",
                               capi.sqlite3_js_rc_str(rc));
    }finally{
       if( pData && pData!==data ) wasm.dealloc(pData);
    }
  };

  /**
     Converts SQL input from a variety of convenient formats
     to plain strings.

     If v is a string, it is returned as-is. If it is-a Array, its
     join("") result is returned.  If is is a Uint8Array, Int8Array,
     or ArrayBuffer, it is assumed to hold UTF-8-encoded text and is
     decoded to a string. If it looks like a WASM pointer,
     wasm.cstrToJs(sql) is returned. Else undefined is returned.

     Added in 3.44
  */
  capi.sqlite3_js_sql_to_string = (sql)=>{
    if('string' === typeof sql){
      return sql;
    }
    const x = flexibleString(v);
    return x===v ? undefined : x;
  }

  /**
     Wraps all known variants of the C-side variadic
     sqlite3_db_config().

     Full docs: https://sqlite.org/c3ref/db_config.html

     Returns capi.SQLITE_MISUSE if op is not a valid operation ID.

     The variants which take `(int, int*)` arguments treat a
     missing or falsy pointer argument as 0.
  */
  capi.sqlite3_db_config = function(pDb, op, ...args){
    switch(op){
      case capi.SQLITE_DBCONFIG_ENABLE_FKEY:
      case capi.SQLITE_DBCONFIG_ENABLE_TRIGGER:
      case capi.SQLITE_DBCONFIG_ENABLE_FTS3_TOKENIZER:
      case capi.SQLITE_DBCONFIG_ENABLE_LOAD_EXTENSION:
      case capi.SQLITE_DBCONFIG_NO_CKPT_ON_CLOSE:
      case capi.SQLITE_DBCONFIG_ENABLE_QPSG:
      case capi.SQLITE_DBCONFIG_TRIGGER_EQP:
      case capi.SQLITE_DBCONFIG_RESET_DATABASE:
      case capi.SQLITE_DBCONFIG_DEFENSIVE:
      case capi.SQLITE_DBCONFIG_WRITABLE_SCHEMA:
      case capi.SQLITE_DBCONFIG_LEGACY_ALTER_TABLE:
      case capi.SQLITE_DBCONFIG_DQS_DML:
      case capi.SQLITE_DBCONFIG_DQS_DDL:
      case capi.SQLITE_DBCONFIG_ENABLE_VIEW:
      case capi.SQLITE_DBCONFIG_LEGACY_FILE_FORMAT:
      case capi.SQLITE_DBCONFIG_TRUSTED_SCHEMA:
      case capi.SQLITE_DBCONFIG_STMT_SCANSTATUS:
      case capi.SQLITE_DBCONFIG_REVERSE_SCANORDER:
      case capi.SQLITE_DBCONFIG_ENABLE_ATTACH_CREATE:
      case capi.SQLITE_DBCONFIG_ENABLE_ATTACH_WRITE:
      case capi.SQLITE_DBCONFIG_ENABLE_COMMENTS:
      case capi.SQLITE_DBCONFIG_FP_DIGITS:
        if( !this.ip ){
          this.ip = wasm.xWrap('sqlite3__wasm_db_config_ip','int',
                               ['sqlite3*', 'int', 'int', '*']);
        }
        return this.ip(pDb, op, args[0], args[1] || 0);
      case capi.SQLITE_DBCONFIG_LOOKASIDE:
        if( !this.pii ){
          this.pii = wasm.xWrap('sqlite3__wasm_db_config_pii', 'int',
                                ['sqlite3*', 'int', '*', 'int', 'int']);
        }
        return this.pii(pDb, op, args[0], args[1], args[2]);
      case capi.SQLITE_DBCONFIG_MAINDBNAME:
        if(!this.s){
          this.s = wasm.xWrap('sqlite3__wasm_db_config_s','int',
                              ['sqlite3*', 'int', 'string:static']
                              /* MAINDBNAME requires a static string */);
        }
        return this.s(pDb, op, args[0]);
      default:
        return capi.SQLITE_MISUSE;
    }
  }.bind(nu());

  /**
     Given a (sqlite3_value*), this function attempts to convert it
     to an equivalent JS value with as much fidelity as feasible and
     return it.

     By default it throws if it cannot determine any sensible
     conversion. If passed a falsy second argument, it instead returns
     `undefined` if no suitable conversion is found.  Note that there
     is no conversion from SQL to JS which results in the `undefined`
     value, so `undefined` has an unambiguous meaning here.  It will
     always throw a WasmAllocError if allocating memory for a
     conversion fails.

     Caveats:

     - It does not support sqlite3_value_to_pointer() conversions
       because those require a type name string which this function
       does not have and cannot sensibly be given at the level of the
       API where this is used (e.g. automatically converting UDF
       arguments). Clients using sqlite3_value_to_pointer(), and its
       related APIs, will need to manage those themselves.
  */
  capi.sqlite3_value_to_js = function(pVal,throwIfCannotConvert=true){
    let arg;
    const valType = capi.sqlite3_value_type(pVal);
    switch(valType){
        case capi.SQLITE_INTEGER:
          if(wasm.bigIntEnabled){
            arg = capi.sqlite3_value_int64(pVal);
            if(util.bigIntFitsDouble(arg)) arg = Number(arg);
          }
          else arg = capi.sqlite3_value_double(pVal)/*yes, double, for larger integers*/;
          break;
        case capi.SQLITE_FLOAT:
          arg = capi.sqlite3_value_double(pVal);
          break;
        case capi.SQLITE_TEXT:
          arg = capi.sqlite3_value_text(pVal);
          break;
        case capi.SQLITE_BLOB:{
          const n = capi.sqlite3_value_bytes(pVal);
          const pBlob = capi.sqlite3_value_blob(pVal);
          if(n && !pBlob) sqlite3.WasmAllocError.toss(
            "Cannot allocate memory for blob argument of",n,"byte(s)"
          );
          arg = n
            ? wasm.heap8u().slice(Number(pBlob), Number(pBlob) + Number(n))
            : null;
          break;
        }
        case capi.SQLITE_NULL:
          arg = null; break;
        default:
          if(throwIfCannotConvert){
            toss3(capi.SQLITE_MISMATCH,
                  "Unhandled sqlite3_value_type():",valType);
          }
          arg = undefined;
    }
    return arg;
  };

  /**
     Requires a C-style array of `sqlite3_value*` objects and the
     number of entries in that array. Returns a JS array containing
     the results of passing each C array entry to
     sqlite3_value_to_js(). The 3rd argument to this function is
     passed on as the 2nd argument to that one.
  */
  capi.sqlite3_values_to_js = function(argc,pArgv,throwIfCannotConvert=true){
    let i;
    const tgt = [];
    for(i = 0; i < argc; ++i){
      /**
         Curiously: despite ostensibly requiring 8-byte
         alignment, the pArgv array is parcelled into chunks of
         4 bytes (1 pointer each). The values those point to
         have 8-byte alignment but the individual argv entries
         do not.
      */
      tgt.push(capi.sqlite3_value_to_js(
        wasm.peekPtr(wasm.ptr.add(pArgv, wasm.ptr.size * i)),
        throwIfCannotConvert
      ));
    }
    return tgt;
  };

  /**
     Calls either sqlite3_result_error_nomem(), if e is-a
     WasmAllocError, or sqlite3_result_error(). In the latter case,
     the second argument is coerced to a string to create the error
     message.

     The first argument is a (sqlite3_context*). Returns void.
     Does not throw.
  */
  capi.sqlite3_result_error_js = function(pCtx,e){
    if(e instanceof WasmAllocError){
      capi.sqlite3_result_error_nomem(pCtx);
    }else{
      /* Maintenance reminder: ''+e, rather than e.message,
         will prefix e.message with e.name, so it includes
         the exception's type name in the result. */;
      capi.sqlite3_result_error(pCtx, ''+e, -1);
    }
  };

  /**
     This function passes its 2nd argument to one of the
     sqlite3_result_xyz() routines, depending on the type of that
     argument:

     - If (val instanceof Error), this function passes it to
       sqlite3_result_error_js().
     - `null`: `sqlite3_result_null()`
     - `boolean`: `sqlite3_result_int()` with a value of 0 or 1.
     - `number`: `sqlite3_result_int()`, `sqlite3_result_int64()`, or
       `sqlite3_result_double()`, depending on the range of the number
       and whether or not int64 support is enabled.
     - `bigint`: similar to `number` but will trigger an error if the
       value is too big to store in an int64.
     - `string`: `sqlite3_result_text()`
     - Uint8Array or Int8Array or ArrayBuffer: `sqlite3_result_blob()`
     - `undefined`: is a no-op provided to simplify certain use cases.

     Anything else triggers `sqlite3_result_error()` with a
     description of the problem.

     The first argument to this function is a `(sqlite3_context*)`.
     Returns void. Does not throw.
  */
  capi.sqlite3_result_js = function(pCtx,val){
    if(val instanceof Error){
      capi.sqlite3_result_error_js(pCtx, val);
      return;
    }
    try{
      switch(typeof val) {
          case 'undefined':
            /* This is a no-op. This routine originated in the create_function()
               family of APIs and in that context, passing in undefined indicated
               that the caller was responsible for calling sqlite3_result_xxx()
               (if needed). */
            break;
          case 'boolean':
            capi.sqlite3_result_int(pCtx, val ? 1 : 0);
            break;
          case 'bigint':
            if(util.bigIntFits32(val)){
              capi.sqlite3_result_int(pCtx, Number(val));
            }else if(util.bigIntFitsDouble(val)){
              capi.sqlite3_result_double(pCtx, Number(val));
            }else if(wasm.bigIntEnabled){
              if(util.bigIntFits64(val)) capi.sqlite3_result_int64(pCtx, val);
              else toss3("BigInt value",val.toString(),"is too BigInt for int64.");
            }else{
              toss3("BigInt value",val.toString(),"is too BigInt.");
            }
            break;
          case 'number': {
            let f;
            if(util.isInt32(val)){
              f = capi.sqlite3_result_int;
            }else if(wasm.bigIntEnabled
                     && Number.isInteger(val)
                     && util.bigIntFits64(BigInt(val))){
              f = capi.sqlite3_result_int64;
            }else{
              f = capi.sqlite3_result_double;
            }
            f(pCtx, val);
            break;
          }
          case 'string': {
            const [p, n] = wasm.allocCString(val,true);
            capi.sqlite3_result_text(pCtx, p, n, capi.SQLITE_WASM_DEALLOC);
            break;
          }
          case 'object':
            if(null===val/*yes, typeof null === 'object'*/) {
              capi.sqlite3_result_null(pCtx);
              break;
            }else if(util.isBindableTypedArray(val)){
              const pBlob = wasm.allocFromTypedArray(val);
              capi.sqlite3_result_blob(
                pCtx, pBlob, val.byteLength,
                capi.SQLITE_WASM_DEALLOC
              );
              break;
            }
            // else fall through
          default:
            toss3("Don't not how to handle this UDF result value:",(typeof val), val);
      }
    }catch(e){
      capi.sqlite3_result_error_js(pCtx, e);
    }
  };

  /**
     Returns the result sqlite3_column_value(pStmt,iCol) passed to
     sqlite3_value_to_js(). The 3rd argument of this function is
     ignored by this function except to pass it on as the second
     argument of sqlite3_value_to_js(). If the sqlite3_column_value()
     returns NULL (e.g. because the column index is out of range),
     this function returns `undefined`, regardless of the 3rd
     argument. If the 3rd argument is falsy and conversion fails,
     `undefined` will be returned.

     Note that sqlite3_column_value() returns an "unprotected" value
     object, but in a single-threaded environment (like this one)
     there is no distinction between protected and unprotected values.
  */
  capi.sqlite3_column_js = function(pStmt, iCol, throwIfCannotConvert=true){
    const v = capi.sqlite3_column_value(pStmt, iCol);
    return (0===v) ? undefined : capi.sqlite3_value_to_js(v, throwIfCannotConvert);
  };

  if( true ){ /* changeset/preupdate additions... */
    /**
       Internal impl of sqlite3_preupdate_new/old_js() and
       sqlite3changeset_new/old_js().
    */
    const __newOldValue = function(pObj, iCol, impl){
      impl = capi[impl];
      if(!this.ptr) this.ptr = wasm.allocPtr();
      else wasm.pokePtr(this.ptr, 0);
      const rc = impl(pObj, iCol, this.ptr);
      if(rc) return SQLite3Error.toss(rc,arguments[2]+"() failed with code "+rc);
      const pv = wasm.peekPtr(this.ptr);
      return pv ? capi.sqlite3_value_to_js( pv, true ) : undefined;
    }.bind(nu());

    /**
       A wrapper around sqlite3_preupdate_new() which fetches the
       sqlite3_value at the given index and returns the result of
       passing it to sqlite3_value_to_js(). Throws on error.
    */
    capi.sqlite3_preupdate_new_js =
      (pDb, iCol)=>__newOldValue(pDb, iCol, 'sqlite3_preupdate_new');

    /**
       The sqlite3_preupdate_old() counterpart of
       sqlite3_preupdate_new_js(), with an identical interface.
    */
    capi.sqlite3_preupdate_old_js =
      (pDb, iCol)=>__newOldValue(pDb, iCol, 'sqlite3_preupdate_old');

    /**
       A wrapper around sqlite3changeset_new() which fetches the
       sqlite3_value at the given index and returns the result of
       passing it to sqlite3_value_to_js(). Throws on error.

       If sqlite3changeset_new() succeeds but has no value to report,
       this function returns the undefined value, noting that
       undefined is not a valid conversion from an `sqlite3_value`, so
       is unambiguous.
    */
    capi.sqlite3changeset_new_js =
      (pChangesetIter, iCol) => __newOldValue(pChangesetIter, iCol,
                                              'sqlite3changeset_new');

    /**
       The sqlite3changeset_old() counterpart of
       sqlite3changeset_new_js(), with an identical interface.
    */
    capi.sqlite3changeset_old_js =
      (pChangesetIter, iCol)=>__newOldValue(pChangesetIter, iCol,
                                            'sqlite3changeset_old');
  }/*changeset/preupdate additions*/

  /**
     sqlite3_js_retry_busy(maxTimes,callback[,beforeRetry])

     Calls the given _synchronous_ callback function. If that function
     returns sqlite3.capi.SQLITE_BUSY _or_ throws an SQLite3Error
     with a resultCode property of that value then it will suppress
     that error and try again, up to the given maximum number of
     times. If the callback returns any other value than that,
     it is returned. If the maximum number of retries has been
     reached, an SQLite3Error with a resultCode value of
     sqlite3.capi.SQLITE_BUSY is thrown. If the callback throws any
     exception other than the aforementioned BUSY exception, it is
     propagated. If it throws a BUSY exception on its final attempt,
     that is propagated as well.

     If the beforeRetry argument is given, it must be a _synchronous_
     function.  It is called immediately before each retry of the
     callback (not for the initial call), passed the attempt number
     (so it starts with 2, not 1). If it throws, the exception is
     handled as described above. Its result value is ignored.

     To effectively retry "forever", pass a huge maxTimes value such
     as Number.MAX_SAFE_INTEGER, with the caveat that there is no
     recovery from that unless the beforeRetry() can figure out when
     to throw.

     Added in 3.53.0.

     TODO?: an async variant of this.
  */
  capi.sqlite3_js_retry_busy = function(maxTimes, callback, beforeRetry){
    for(let n = 1; n <= maxTimes; ++n){
      try{
        if( beforeRetry && n>1 ) beforeRetry(n);
        const rc = callback();
        if( capi.SQLITE_BUSY===rc ){
          if( n===maxTimes ){
            throw new SQLite3Error(rc, [
              "sqlite3_js_retry_busy() max retry attempts (",
              maxTimes,
              ") reached."
            ].join(''));
          }
          continue;
        }
        return rc;
      }catch(e){
        if( n<maxTimes
            && (e instanceof SQLite3Error)
            && e.resultCode===capi.SQLITE_BUSY ){
          continue;
        }
        throw e;
      }
    }
  };

  /* The remainder of the API will be set up in later steps. */
  const sqlite3 = {
    WasmAllocError: WasmAllocError,
    SQLite3Error: SQLite3Error,
    capi,
    util /* internal: will get removed after library init */,
    wasm,
    config,
    /**
       Holds the version info of the sqlite3 source tree from which
       the generated sqlite3-api.js gets built. Its version may well
       differ from that reported by sqlite3_libversion(), but that
       should be considered a source file mismatch, as the JS and WASM
       files are intended to be built and distributed together.

       This object is initially a placeholder which gets replaced by a
       build-generated object.
    */
    version: nu(),

    /**
       The library reserves the 'client' property for client-side use
       and promises to never define a property with this name nor to
       ever rely on specific contents of it. It makes no such guarantees
       for other properties.
    */
    client: undefined,

    /**
       This function is not part of the public interface, but a
       piece of internal bootstrapping infrastructure.

       Performs any optional asynchronous library-level initialization
       which might be required. This function returns a Promise which
       resolves to the sqlite3 namespace object. Any error in the
       async init will be fatal to the init as a whole, but init
       routines are themselves welcome to install dummy catch()
       handlers which are not fatal if their failure should be
       considered non-fatal.

       Ideally this function is called as part of the Promise chain
       which handles the loading and bootstrapping of the API.  If not
       then it must be called by client-level code, which must not use
       the library until the returned Promise resolves.

       If called multiple times it will return the same Promise on
       subsequent calls. The current build setup precludes that
       possibility, so it's only a hypothetical problem if/when this
       function ever needs to be invoked by clients.

       In Emscripten-based builds, this function is called
       automatically and deleted from this object.
    */
    asyncPostInit: async function ff(){
      if(ff.isReady instanceof Promise) return ff.isReady;
      let lia = this.initializersAsync;
      delete this.initializersAsync;
      const postInit = async ()=>{
        if(!sqlite3.__isUnderTest){
          /* Delete references to internal-only APIs which are used by
             some initializers. Retain them when running in test mode
             so that we can add tests for them. */
          delete sqlite3.util;
          delete sqlite3.StructBinder;
          delete sqlite3.opfs;
        }
        return sqlite3;
      };
      const catcher = (e)=>{
        config.error("an async sqlite3 initializer failed:",e);
        throw e;
      };
      if(!lia || !lia.length){
        return ff.isReady = postInit().catch(catcher);
      }
      lia = lia.map((f)=>{
        return (f instanceof Function) ? async x=>f(sqlite3) : f;
      });
      lia.push(postInit);
      let p = Promise.resolve(sqlite3);
      while(lia.length) p = p.then(lia.shift());
      return ff.isReady = p.catch(catcher);
    }.bind(sqlite3ApiBootstrap),
    /**
       scriptInfo holds information about the currenty-loading script
       so that we can locate the WASM file if it's somewhere other
       than the build-time-defined directory. It ideally gets injected
       into this object by the infrastructure which assembles the
       JS/WASM module. It contains state which must be collected
       before sqlite3ApiBootstrap() can be declared. It is not
       necessarily available to any sqlite3ApiBootstrap.initializers
       but "should" be in place (if it's added at all) by the time
       that sqlite3ApiBootstrap.initializersAsync is processed.

       This state is not part of the public API, only intended for use
       with the sqlite3 API bootstrapping and wasm-loading process.
    */
    scriptInfo: undefined
  };
  if( 'undefined'!==typeof sqlite3IsUnderTest/* from post-js-header.js */ ){
    sqlite3.__isUnderTest = !!sqlite3IsUnderTest;
  }
  try{
    sqlite3ApiBootstrap.initializers.forEach((f)=>{
      f(sqlite3);
    });
  }catch(e){
    /* If we don't report this here, it can get completely swallowed
       up and disappear into the abyss of Promises and Workers. */
    console.error("sqlite3 bootstrap initializer threw:",e);
    throw e;
  }
  delete sqlite3ApiBootstrap.initializers;
  sqlite3ApiBootstrap.sqlite3 = sqlite3;
  if( 'undefined'!==typeof sqlite3InitScriptInfo/* from post-js-header.js */ ){
    sqlite3InitScriptInfo.debugModule(
      "sqlite3ApiBootstrap() complete", sqlite3
    );
    sqlite3.scriptInfo
    /* Used by some async init code. As of 2025-11-15 this is still
       in use by the OPFS VFS for locating its worker. In non-Emscripten
       builds, this would need to be injected in somewhere to get
       that VFS loading. */ = sqlite3InitScriptInfo;
  }
  if( sqlite3.__isUnderTest ){
    if( 'undefined'!==typeof EmscriptenModule ){
      sqlite3.config.emscripten = EmscriptenModule;
    }
    /*
      The problem with exposing these pieces (in non-testing runs) via
      sqlite3.wasm is that it exposes non-SQLite pieces to the
      clients, who may come to expect it to remain. _We_ only have
      these data because we've overridden Emscripten's wasm file
      loader, and if we lose that capability for some reason then
      we'll lose access to this metadata.

      These data are interesting for exploring how the wasm/JS pieces
      connect, e.g. for exploring exactly what Emscripten imports into
      WASM from its JS glue, but it's not SQLite-related and is not
      required for the library to work.
    */
    const iw = sqlite3.scriptInfo?.instantiateWasm;
    if( iw ){
      /* Metadata injected by the custom Module.instantiateWasm()
         in pre-js.c-pp.js. */
      sqlite3.wasm.module = iw.module;
      sqlite3.wasm.instance = iw.instance;
      sqlite3.wasm.imports = iw.imports;
    }
  }

  /**
     Eliminate any confusion about whether these config objects may
     be used after library initialization by eliminating the outward-facing
     objects...
  */
  delete globalThis.sqlite3ApiConfig;
  delete globalThis.sqlite3ApiBootstrap;
  delete sqlite3ApiBootstrap.defaultConfig;
  return sqlite3.asyncPostInit().then((s)=>{
    if( 'undefined'!==typeof sqlite3InitScriptInfo/* from post-js-header.js */ ){
      sqlite3InitScriptInfo.debugModule(
        "sqlite3.asyncPostInit() complete", s
      );
    }
    delete s.asyncPostInit;
    delete s.scriptInfo;
    delete s.emscripten;
    return s;
  });
}/*sqlite3ApiBootstrap()*/;

/**
  globalThis.sqlite3ApiBootstrap.initializers is an internal detail
  used by the various pieces of the sqlite3 API's amalgamation
  process. It must not be modified by client code except when plugging
  such code into the amalgamation process.

  Each component of the amalgamation is expected to append a function
  to this array. When sqlite3ApiBootstrap() is called for the first
  time, each such function will be called (in their appended order)
  and passed the sqlite3 namespace object, into which they can install
  their features. At the end of that process, this array is deleted.

  The order of insertion into this array is significant for
  some pieces. e.g. sqlite3.capi and sqlite3.wasm cannot be fully
  utilized until the whwasmutil.js part is plugged in via
  sqlite3-api-glue.js.
*/
globalThis.sqlite3ApiBootstrap.initializers = [];

/**
  globalThis.sqlite3ApiBootstrap.initializersAsync is an internal detail
  used by the sqlite3 API's amalgamation process. It must not be
  modified by client code except when plugging such code into the
  amalgamation process.

  The counterpart of globalThis.sqlite3ApiBootstrap.initializers,
  specifically for initializers which are asynchronous. All entries in
  this list must be either async functions, non-async functions which
  return a Promise, or a Promise. Each function in the list is called
  with the sqlite3 object as its only argument.

  The resolved value of any Promise is ignored and rejection will kill
  the asyncPostInit() process (at an indeterminate point because all
  of them are run asynchronously in parallel).

  This list is not processed until the client calls
  sqlite3.asyncPostInit(). This means, for example, that intializers
  added to globalThis.sqlite3ApiBootstrap.initializers may push entries to
  this list.
*/
globalThis.sqlite3ApiBootstrap.initializersAsync = [];

/**
   Client code may assign sqlite3ApiBootstrap.defaultConfig an
   object-type value before calling sqlite3ApiBootstrap() (without
   arguments) in order to tell that call to use this object as its
   default config value. The intention of this is to provide
   downstream clients with a reasonably flexible approach for plugging in
   an environment-suitable configuration without having to define a new
   global-scope symbol.
*/
globalThis.sqlite3ApiBootstrap.defaultConfig = Object.create(null);

/**
   Placeholder: gets installed by the first call to
   globalThis.sqlite3ApiBootstrap(). However, it is recommended that the
   caller of sqlite3ApiBootstrap() capture its return value and delete
   globalThis.sqlite3ApiBootstrap after calling it. It returns the same
   value which will be stored here.
*/
globalThis.sqlite3ApiBootstrap.sqlite3 = undefined;
globalThis.sqlite3ApiBootstrap.initializers.push(function(sqlite3){
  sqlite3.version = {"libVersion": "3.53.3", "libVersionNumber": 3053003, "sourceId": "2026-06-26 20:14:12 d4c0e51e4aeb96955b99185ab9cde75c339e2c29c3f3f12428d364a10d782c62","downloadVersion": 3530300,"scm":{ "sha3-256": "d4c0e51e4aeb96955b99185ab9cde75c339e2c29c3f3f12428d364a10d782c62","branch": "branch-3.53","tags": "release version-3.53.3","datetime": "2026-06-26T20:14:12.354Z"}};
});
/**
  2022-07-08

  The author disclaims copyright to this source code.  In place of a
  legal notice, here is a blessing:

  *   May you do good and not evil.
  *   May you find forgiveness for yourself and forgive others.
  *   May you share freely, never taking more than you give.

  ***********************************************************************

  This code is developed in conjunction with the Jaccwabyt project:

  https://fossil.wanderinghorse.net/r/jaccwabyt

  More specifically:

  https://fossil.wanderinghorse.net/r/jaccwabyt/dir/wasmutil

  and SQLite:

  https://sqlite.org

  This file is kept in sync between both of those trees.

  This build was generated using:

  ./c-pp -o js/whwasmutil.js -@policy=error wasmutil/whwasmutil.c-pp.js

  by libcmpp 2.x 2fc4afc31f6505c27b9c34988973a2bd9b157d559247cdd26868ae75632c3a5e @ 2025-11-16 23:03:27.352 UTC
*/
/**
   The primary goal of this function is to provide JS/WASM utility
   code similar to some of that provided by Emscripten-generated
   builds, the difference being that this one can be used in arbitrary
   WASM environments built with toolchains other than Emscripten. To
   that end, it populates the given object with various WASM-specific
   APIs. These APIs work with both 32- and 64-bit WASM builds.

   Forewarning: this API explicitly targets only browser environments.
   If a given non-browser environment has the capabilities needed for
   a given feature (e.g. TextEncoder), great, but it does not go out
   of its way to account for them and does not provide compatibility
   crutches for them. That said: no specific incompatibilities with,
   e.g., node.js are known (whereas it is known that some folks
   use this with node.js).

   Intended usage:

   ```
   const target = {}; // ... some object ...
   globalThis.WhWasmUtilInstaller(target);
   delete globalThis.WhWasmUtilInstaller;
   ```

   The `target` object then holds the APIs. The caller may set certain
   properties on it, before calling this, to configure it, as
   documented below.

   The global-scope symbol for this function is intended only to
   provide an easy way to make it available to 3rd-party scripts and
   "should" be deleted after calling it. That symbol is _not_ used
   within the library.

   It currently offers alternatives to the following
   Emscripten-generated APIs:

   - OPTIONALLY memory allocation, but how this gets imported is
     environment-specific.  Most of the following features only work
     if allocation is available.

   - WASM-exported "indirect function table" access and
     manipulation. e.g.  creating new WASM-side functions using JS
     functions, analog to Emscripten's addFunction() and
     uninstallFunction() but slightly different and with more useful
     lifetime semantics.

   - Get/set specific heap memory values, analog to Emscripten's
     getValue() and setValue().

   - String length counting in UTF-8 bytes (C-style and JS strings).

   - JS string to C-string conversion and vice versa, analog to
     Emscripten's stringToUTF8Array() and friends, but with slighter
     different interfaces.

   - JS string to Uint8Array conversion, noting that browsers actually
     already have this built in via TextEncoder.

   - "Scoped" allocation, such that allocations made inside of a given
     explicit scope will be automatically cleaned up when the scope is
     closed. This is fundamentally similar to Emscripten's
     stackAlloc() and friends but uses the heap instead of the stack
     because access to the stack requires C code.

   - Create JS wrappers for WASM functions, analog to Emscripten's
     ccall() and cwrap() functions, except that the automatic
     conversions for function arguments and return values can be
     easily customized by the client by assigning custom function
     signature type names to conversion functions. Essentially,
     it's ccall() and cwrap() on steroids.

   How to install...

   Passing an object to this function will install this library's
   functionality into that object. It returns its argument.

   After installation, client code "should" delete this function's
   global symbol (if any).

   This code requires that the target object have the following
   properties, though they needn't be available until the first time
   one of the installed APIs is used (as opposed to when this function
   is called) except where explicitly noted:

   - `exports` must be a property of the target object OR a property
     of `target.instance` (a WebAssembly.Module instance) and it must
     contain the symbols exported by the WASM module associated with
     this code. In an Enscripten environment it must be set to
     `Module['asm']` (versions <=3.1.43) or `wasmExports` (versions
     >=3.1.44). The exports object must contain a minimum of the
     following symbols:

     - `memory`: a WebAssembly.Memory object representing the WASM
       memory. _Alternately_, the `memory` property can be set as
       `target.memory`, in particular if the WASM heap memory is
       initialized in JS an _imported_ into WASM, as opposed to being
       initialized in WASM and exported to JS.

     - `__indirect_function_table`: the WebAssembly.Table object which
       holds WASM-exported functions. This API does not strictly
       require that the table be able to grow but it will throw if its
       `installFunction()` is called and the table cannot grow.

   - `functionTable`: WebAssembly.Table object holding the indirect
     function call table.  If not set then
     `exports.__indirect_function_table` is assumed. Achtung: this
     property gets replaced by a function with the same name (because
     this API used that name before this config option was added).

   In order to simplify downstream usage, if `target.exports` is not
   set when this is called then a property access interceptor
   (read-only, configurable, enumerable) gets installed as `exports`
   which resolves to `target.instance.exports`, noting that the latter
   property need not exist until the first time `target.exports` is
   accessed.

   Some APIs _optionally_ make use of the `bigIntEnabled` property of
   the target object. It "should" be set to true if the WASM
   environment is compiled with BigInt support, else it must be
   false. If it is false, certain BigInt-related features will trigger
   an exception if invoked. This property, if not set when this is
   called, will get a default value of true only if the BigInt64Array
   constructor is available, else it will default to false.  Having
   the BigInt type is not sufficient for full int64 integration with
   WASM: the target WASM file must also have been built with that
   support. In Emscripten that's done using the `-sWASM_BIGINT` flag.

   Some optional APIs require that the target have the following
   methods:

   - 'alloc()` must behave like C's `malloc()`, allocating N bytes of
     memory and returning its pointer. In Emscripten this is
     conventionally made available via `Module['_malloc']`. This API
     requires that the alloc routine throw on allocation error, as
     opposed to returning null or 0.

   - 'dealloc()` must behave like C's `free()`, accepting either a
     pointer returned from its allocation counterpart or the values
     null/0 (for which it must be a no-op). In Emscripten this is
     conventionally made available via `Module['_free']`.

   APIs which require allocation routines are explicitly documented as
   such and/or have "alloc" in their names.

   Optional configuration values which may be set on target before
   calling this:

   - `pointerIR`: an IR-format string for the WASM environment's
      pointer size. If set it must be either 'i32' or 'i64'. If not
      set, it gets set to whatever this code thinks the pointer size
      is.  Modifying it after this call has no effect. A reliable
      way to get this value is (typeof X()), where X is a function
      from target.exports which returns an innocuous pointer.

   - `pointerSize`: if set, it must be one of 4 or 8 and must
      correspond to the value of `pointerIR`. If not set, it gets set
      to whatever this code thinks the pointer size is (4 unless
      `pointerIR` is 'i64'). If `pointerSize` is set but `pointerIR`
      is not, `pointerIR` gets set appropriately, and vice versa.

   When building with Emscripten's -sMEMORY64=1, `pointerIR` must be
   set to 'i64' and/or `pointerSize` must be set to 8.

   After calling this, the pointerIR and pointerSize properties are
   replaced with a read-only Object member named target.ptr. It
   contains the following read-only helper methods and properties to
   assist in using WASM pointers without having to know what type they
   are:

   - `size` = pointerSize

   - `ir` = pointerIR

   - `null` = a "null" pointer of type Number or BigInt. Equivalent to
     one of Number(0) or BigInt(0). This value compares === to
     WASM NULL pointers.

   - `coerce(arg)` = equivalent to one of Number(arg) or BigInt(arg||0).

   - `add(...args)` = performs "pointer arithmetic" (`wasmPtr+offset`
     does not work in 64-bit builds unless all operands are of type
     BigInt). Adds up all of its arguments, accounting for whether
     each is a Number of BigInt, and returns either a Number or
     BigInt.

   - `addn(...args)` = like `add()` but always returns its result as a
     Number. Equivalent to Number(add(...)).

   ------------------------------------------------------------
   Design notes:

   - This function should probably take a config object and return the
     newly-created (or config-provided) target. The current approach
     seemed better at the time.
*/
'use strict';
globalThis.WhWasmUtilInstaller =
function WhWasmUtilInstaller(target){
  'use strict';
  if(undefined===target.bigIntEnabled){
    target.bigIntEnabled = !!globalThis['BigInt64Array'];
  }

  /** Throws a new Error, the message of which is the concatenation of
      all args with a space between each. */
  const toss = (...args)=>{throw new Error(args.join(' '))};

  if( !target.pointerSize && !target.pointerIR
      && target.alloc && target.dealloc ){
    /* Try to determine the pointer size by allocating. */
    const ptr = target.alloc(1);
    target.pointerSize = ('bigint'===typeof ptr ? 8 : 4);
    target.dealloc(ptr);
  }

  /**
     As of 2025-09-21, this library works with 64-bit WASM modules
     built with Emscripten's -sMEMORY64=1.
  */
  if( target.pointerSize && !target.pointerIR ){
    target.pointerIR = (4===target.pointerSize ? 'i32' : 'i64');
  }
  const __ptrIR = (target.pointerIR ??= 'i32');
  const __ptrSize = (target.pointerSize ??=
                     ('i32'===__ptrIR ? 4 : ('i64'===__ptrIR ? 8 : 0)));
  delete target.pointerSize;
  delete target.pointerIR;

  if( 'i32'!==__ptrIR && 'i64'!==__ptrIR ){
    toss("Invalid pointerIR:",__ptrIR);
  }else if( 8!==__ptrSize && 4!==__ptrSize ){
    toss("Invalid pointerSize:",__ptrSize);
  }

  /** Either BigInt or, if !target.bigIntEnabled, a function which
      throws complaining that BigInt is not enabled. */
  const __BigInt = target.bigIntEnabled
        ? (v)=>BigInt(v || 0)
        : (v)=>toss("BigInt support is disabled in this build.");

  const __Number = (v)=>Number(v||0)/*treat undefined the same as null*/;

  /**
     If target.ptr.ir==='i32' then this is equivalent to
     Number(v||0) else it's equivalent to BigInt(v||0), throwing
     if BigInt support is disabled.

     Why? Because Number(null)===0, but BigInt(null) throws.  We
     perform the same for Number to allow the undefined value to be
     treated as a NULL WASM pointer, primarily to reduce friction in
     many SQLite3 bindings which have long relied on that.
  */
  const __asPtrType = (4===__ptrSize) ? __Number : __BigInt;

  /**
     The number 0 as either type Number or BigInt, depending on
     target.ptr.ir.
  */
  const __NullPtr = __asPtrType(0);

  /**
     Expects any number of numeric arguments, each one of either type
     Number or BigInt. It sums them up (from an implicit starting
     point of 0 or 0n) and returns them as a number of the same type
     which target.ptr.coerce() uses.

     This is a workaround for not being able to mix Number/BigInt in
     addition/subtraction expressions (which we frequently need for
     calculating pointer offsets).
  */
  const __ptrAdd = function(...args){
    let rc = __asPtrType(0);
    for(const v of args) rc += __asPtrType(v);
    return rc;
  };

  /** Set up target.ptr... */
  {
    const __ptr = Object.create(null);
    Object.defineProperty(target, 'ptr', {
      enumerable: true,
      get: ()=>__ptr,
      set: ()=>toss("The ptr property is read-only.")
    });
    (function f(name, val){
      Object.defineProperty(__ptr, name, {
        enumerable: true,
        get: ()=>val,
        set: ()=>toss("ptr["+name+"] is read-only.")
      });
      return f;
    })(
      'null', __NullPtr
    )(
      'size', __ptrSize
    )(
      'ir', __ptrIR
    )(
      'coerce', __asPtrType
    )(
      'add', __ptrAdd
    )(
      'addn', (4===__ptrIR) ? __ptrAdd : (...args)=>Number(__ptrAdd(...args))
    );
  }

  if(!target.exports){
    Object.defineProperty(target, 'exports', {
      enumerable: true, configurable: true,
      get: ()=>(target.instance?.exports)
    });
  }

  /** Stores various cached state. */
  const cache = Object.create(null);
  /** Previously-recorded size of cache.memory.buffer, noted so that
      we can recreate the view objects if the heap grows. */
  cache.heapSize = 0;
  /** WebAssembly.Memory object extracted from target.memory or
      target.exports.memory the first time heapWrappers() is
      called. */
  cache.memory = null;
  /** uninstallFunction() puts table indexes in here for reuse and
      installFunction() extracts them. */
  cache.freeFuncIndexes = [];
  /**
     List-of-lists used by scopedAlloc() and friends.
  */
  cache.scopedAlloc = [];

  /** Push the pointer ptr to the current cache.scopedAlloc list
      (which must already exist) and return ptr. */
  cache.scopedAlloc.pushPtr = (ptr)=>{
    cache.scopedAlloc[cache.scopedAlloc.length-1].push(ptr);
    return ptr;
  };

  cache.utf8Decoder = new TextDecoder();
  cache.utf8Encoder = new TextEncoder('utf-8');

  /**
     For the given IR-like string in the set ('i8', 'i16', 'i32',
     'f32', 'float', 'i64', 'f64', 'double', '*'), or any string value
     ending in '*', returns the sizeof for that value
     (target.ptr.size in the latter case). For any other value, it
     returns the undefined value.
  */
  target.sizeofIR = (n)=>{
    switch(n){
        case 'i8': return 1;
        case 'i16': return 2;
        case 'i32': case 'f32': case 'float': return 4;
        case 'i64': case 'f64': case 'double': return 8;
        case '*': return __ptrSize;
        default:
          return (''+n).endsWith('*') ? __ptrSize : undefined;
    }
  };

  /**
     If (cache.heapSize !== cache.memory.buffer.byteLength), i.e. if
     the heap has grown since the last call, updates cache.HEAPxyz.
     Returns the cache object.
  */
  const heapWrappers = function(){
    if(!cache.memory){
      cache.memory = (target.memory instanceof WebAssembly.Memory)
        ? target.memory : target.exports.memory;
    }else if(cache.heapSize === cache.memory.buffer.byteLength){
      return cache;
    }
    // heap is newly-acquired or has been resized....
    const b = cache.memory.buffer;
    cache.HEAP8 = new Int8Array(b); cache.HEAP8U = new Uint8Array(b);
    cache.HEAP16 = new Int16Array(b); cache.HEAP16U = new Uint16Array(b);
    cache.HEAP32 = new Int32Array(b); cache.HEAP32U = new Uint32Array(b);
    cache.HEAP32F = new Float32Array(b); cache.HEAP64F = new Float64Array(b);
    if(target.bigIntEnabled){
      if( 'undefined'!==typeof BigInt64Array ){
        cache.HEAP64 = new BigInt64Array(b); cache.HEAP64U = new BigUint64Array(b);
      }else{
        toss("BigInt support is enabled, but the BigInt64Array type is missing.");
      }
    }
    cache.heapSize = b.byteLength;
    return cache;
  };

  /** Convenience equivalent of this.heapForSize(8,false). */
  target.heap8 = ()=>heapWrappers().HEAP8;

  /** Convenience equivalent of this.heapForSize(8,true). */
  target.heap8u = ()=>heapWrappers().HEAP8U;

  /** Convenience equivalent of this.heapForSize(16,false). */
  target.heap16 = ()=>heapWrappers().HEAP16;

  /** Convenience equivalent of this.heapForSize(16,true). */
  target.heap16u = ()=>heapWrappers().HEAP16U;

  /** Convenience equivalent of this.heapForSize(32,false). */
  target.heap32 = ()=>heapWrappers().HEAP32;

  /** Convenience equivalent of this.heapForSize(32,true). */
  target.heap32u = ()=>heapWrappers().HEAP32U;

  /**
     Requires n to be one of:

     - integer 8, 16, or 32.
     - A integer-type TypedArray constructor: Int8Array, Int16Array,
     Int32Array, or their Uint counterparts.

     If this.bigIntEnabled is true, it also accepts the value 64 or a
     BigInt64Array/BigUint64Array, else it throws if passed 64 or one
     of those constructors.

     Returns an integer-based TypedArray view of the WASM heap memory
     buffer associated with the given block size. If passed an integer
     as the first argument and unsigned is truthy then the "U"
     (unsigned) variant of that view is returned, else the signed
     variant is returned. If passed a TypedArray value, the 2nd
     argument is ignored. Float32Array and Float64Array views are not
     supported by this function.

     Growth of the heap will invalidate any references to this heap,
     so do not hold a reference longer than needed and do not use a
     reference after any operation which may allocate. Instead,
     re-fetch the reference by calling this function again.

     Throws if passed an invalid n.
  */
  target.heapForSize = function(n,unsigned = true){
    let ctor;
    const c = (cache.memory && cache.heapSize === cache.memory.buffer.byteLength)
          ? cache : heapWrappers();
    switch(n){
        case Int8Array: return c.HEAP8; case Uint8Array: return c.HEAP8U;
        case Int16Array: return c.HEAP16; case Uint16Array: return c.HEAP16U;
        case Int32Array: return c.HEAP32; case Uint32Array: return c.HEAP32U;
        case 8:  return unsigned ? c.HEAP8U : c.HEAP8;
        case 16: return unsigned ? c.HEAP16U : c.HEAP16;
        case 32: return unsigned ? c.HEAP32U : c.HEAP32;
        case 64:
          if(c.HEAP64) return unsigned ? c.HEAP64U : c.HEAP64;
          break;
        default:
          if(target.bigIntEnabled){
            if(n===globalThis['BigUint64Array']) return c.HEAP64U;
            else if(n===globalThis['BigInt64Array']) return c.HEAP64;
            break;
          }
    }
    toss("Invalid heapForSize() size: expecting 8, 16, 32,",
         "or (if BigInt is enabled) 64.");
  };

  const __funcTable = target.functionTable;
  delete target.functionTable;

  /**
     Returns the WASM-exported "indirect function table".
  */
  target.functionTable = __funcTable
    ? ()=>__funcTable
    : ()=>target.exports.__indirect_function_table
    /** -----------------^^^^^ "seems" to be a standardized export name.
        From Emscripten release notes from 2020-09-10:
        - Use `__indirect_function_table` as the import name for the
        table, which is what LLVM does.

        We must delay access to target.exports until after the library
        is bootstrapped.
    */;

  /**
     Given a function pointer, returns the WASM function table entry
     if found, else returns a falsy value: undefined if fptr is out of
     range or null if it's in range but the table entry is empty.
  */
  target.functionEntry = function(fptr){
    const ft = target.functionTable();
    //console.debug("functionEntry(",arguments,")", __asPtrType(fptr));
    //-sMEMORY64=1: we get a BigInt fptr and ft.get() wants a BigInt.
    //-sMEMORY64=2: we get a Number fptr and ft.get() wants a Number.
    return fptr < ft.length ? ft.get(__asPtrType(fptr)) : undefined;
  };

  /**
     Creates a WASM function which wraps the given JS function and
     returns the JS binding of that WASM function. The signature
     string must be the Jaccwabyt-format or Emscripten
     addFunction()-format function signature string. In short: in may
     have one of the following formats:

     - Emscripten: `"x..."`, where the first x is a letter representing
       the result type and subsequent letters represent the argument
       types. Functions with no arguments have only a single
       letter.

     - Jaccwabyt: `"x(...)"` where `x` is the letter representing the
       result type and letters in the parens (if any) represent the
       argument types. Functions with no arguments use `x()`.

     Supported letters:

     - `i` = int32
     - `p` = int32 or int64 ("pointer"), depending on target.ptr.size
     - `j` = int64
     - `f` = float32
     - `d` = float64
     - `v` = void, only legal for use as the result type

     It throws if an invalid signature letter is used.

     Jaccwabyt-format signatures support some additional letters which
     have no special meaning here but (in this context) act as aliases
     for other letters:

     - `s`, `P`: same as `p`

     Sidebar: this code is developed together with Jaccwabyt, thus the
     support for its signature format.

     The arguments may be supplied in either order: (func,sig) or
     (sig,func).
  */
  target.jsFuncToWasm = function f(func, sig){
    /** Attribution: adapted up from Emscripten-generated glue code,
        refactored primarily for efficiency's sake, eliminating
        call-local functions and superfluous temporary arrays. */
    if(!f._){/*static init...*/
      f._ = {
        /* Map of signature letters to type IR values */
        sigTypes: Object.assign(Object.create(null),{
          i: 'i32', p: __ptrIR, P: __ptrIR, s: __ptrIR,
          j: 'i64', f: 'f32', d: 'f64'
        }),

        /* Map of type IR values to WASM type code values */
        typeCodes: Object.assign(Object.create(null),{
          f64: 0x7c, f32: 0x7d, i64: 0x7e, i32: 0x7f
        }),

        /** Encodes n, which must be <2^14 (16384), into target array
            tgt, as a little-endian value, using the given method
            ('push' or 'unshift'). */
        uleb128Encode: (tgt, method, n)=>{
          if(n<128) tgt[method](n);
          else tgt[method]( (n % 128) | 128, n>>7);
        },

        /** Intentionally-lax pattern for Jaccwabyt-format function
            pointer signatures, the intent of which is simply to
            distinguish them from Emscripten-format signatures. The
            downstream checks are less lax. */
        rxJSig: /^(\w)\((\w*)\)$/,

        /** Returns the parameter-value part of the given signature
            string. */
        sigParams: (sig)=>{
          const m = f._.rxJSig.exec(sig);
          return m ? m[2] : sig.substr(1);
        },

        /** Returns the IR value for the given letter or throws
            if the letter is invalid. */
        letterType: (x)=>f._.sigTypes[x] || toss("Invalid signature letter:",x),

        /** Pushes the WASM data type code for the given signature
            letter to the given target array. Throws if letter is
            invalid. */
        pushSigType: (dest, letter)=>dest.push(f._.typeCodes[f._.letterType(letter)])

        /** Returns an object describing the result type and parameter
            type(s) of the given function signature, or throws if the
            signature is invalid. */
        /******** // only valid for use with the WebAssembly.Function ctor, which
                  // is not yet documented on MDN.
        sigToWasm: function(sig){
          const rc = {parameters:[], results: []};
          if('v'!==sig[0]) rc.results.push(f.sigTypes(sig[0]));
          for(const x of f._.sigParams(sig)){
            rc.parameters.push(f._.typeCodes(x));
          }
          return rc;
        },************/
      };
    }/*static init*/
    if('string'===typeof func){
      const x = sig;
      sig = func;
      func = x;
    }
    const _ = f._;
    const sigParams = _.sigParams(sig);
    const wasmCode = [0x01/*count: 1*/, 0x60/*function*/];
    _.uleb128Encode(wasmCode, 'push', sigParams.length);
    for(const x of sigParams) _.pushSigType(wasmCode, x);
    if('v'===sig[0]) wasmCode.push(0);
    else{
      wasmCode.push(1);
      _.pushSigType(wasmCode, sig[0]);
    }
    _.uleb128Encode(wasmCode, 'unshift', wasmCode.length)/* type section length */;
    wasmCode.unshift(
      0x00, 0x61, 0x73, 0x6d, /* magic: "\0asm" */
      0x01, 0x00, 0x00, 0x00, /* version: 1 */
      0x01 /* type section code */
    );
    wasmCode.push(
      /* import section: */ 0x02, 0x07,
      /* (import "e" "f" (func 0 (type 0))): */
      0x01, 0x01, 0x65, 0x01, 0x66, 0x00, 0x00,
      /* export section: */ 0x07, 0x05,
      /* (export "f" (func 0 (type 0))): */
      0x01, 0x01, 0x66, 0x00, 0x00
    );
    return (new WebAssembly.Instance(
      new WebAssembly.Module(new Uint8Array(wasmCode)), {
        e: { f: func }
      })).exports['f'];
  }/*jsFuncToWasm()*/;

  /**
     Documented as target.installFunction() except for the 3rd
     argument: if truthy, the newly-created function pointer
     is stashed in the current scoped-alloc scope and will be
     cleaned up at the matching scopedAllocPop(), else it
     is not stashed there.
  */
  const __installFunction = function f(func, sig, scoped){
    if(scoped && !cache.scopedAlloc.length){
      toss("No scopedAllocPush() scope is active.");
    }
    if('string'===typeof func){
      const x = sig;
      sig = func;
      func = x;
    }
    if('string'!==typeof sig || !(func instanceof Function)){
      toss("Invalid arguments: expecting (function,signature) "+
           "or (signature,function).");
    }
    const ft = target.functionTable();
    const oldLen = __asPtrType(ft.length);
    let ptr;
    while( (ptr = cache.freeFuncIndexes.pop()) ){
      if(ft.get(ptr)){
        /* freeFuncIndexes's entry is stale. Table was modified via a
           different API */
        ptr = null;
        continue;
      }else{
        /* This index is free. We'll re-use it. */
        break;
      }
    }
    if(!ptr){
      ptr = __asPtrType(oldLen);
      ft.grow(__asPtrType(1));
    }
    try{
      /*this will only work if func is a WASM-exported function*/
      ft.set(ptr, func);
      if(scoped){
        cache.scopedAlloc.pushPtr(ptr);
      }
      return ptr;
    }catch(e){
      if(!(e instanceof TypeError)){
        if(ptr===oldLen) cache.freeFuncIndexes.push(oldLen);
        throw e;
      }
    }
    // It's not a WASM-exported function, so compile one...
    try {
      const fptr = target.jsFuncToWasm(func, sig);
      ft.set(ptr, fptr);
      if(scoped){
        cache.scopedAlloc.pushPtr(ptr);
      }
    }catch(e){
      if(ptr===oldLen) cache.freeFuncIndexes.push(oldLen);
      throw e;
    }
    return ptr;
  };

  /**
     Expects a JS function and signature, exactly as for
     this.jsFuncToWasm(). It uses that function to create a
     WASM-exported function, installs that function to the next
     available slot of this.functionTable(), and returns the
     function's index in that table (which acts as a pointer to that
     function). The returned pointer can be passed to
     uninstallFunction() to uninstall it and free up the table slot
     for reuse.

     If passed (string,function) arguments then it treats the first
     argument as the signature and second as the function.

     As a special case, if the passed-in function is a WASM-exported
     function then the signature argument is ignored and func is
     installed as-is, without requiring re-compilation/re-wrapping.

     This function will propagate an exception if
     WebAssembly.Table.grow() throws or this.jsFuncToWasm() throws.
     The former case can happen in an Emscripten-compiled environment
     when building without Emscripten's `-sALLOW_TABLE_GROWTH` flag.

     Sidebar: this function differs from Emscripten's addFunction()
     _primarily_ in that it does not share that function's
     undocumented behavior of reusing a function if it's passed to
     addFunction() more than once, which leads to uninstallFunction()
     breaking clients which do not take care to avoid that case:

     https://github.com/emscripten-core/emscripten/issues/17323
  */
  target.installFunction = (func, sig)=>__installFunction(func, sig, false);

  /**
     Works exactly like installFunction() but requires that a
     scopedAllocPush() is active and uninstalls the given function
     when that alloc scope is popped via scopedAllocPop().
     This is used for implementing JS/WASM function bindings which
     should only persist for the life of a call into a single
     C-side function.
  */
  target.scopedInstallFunction = (func, sig)=>__installFunction(func, sig, true);

  /**
     Requires a pointer value previously returned from
     this.installFunction(). Removes that function from the WASM
     function table, marks its table slot as free for re-use, and
     returns that function. It is illegal to call this before
     installFunction() has been called and results are undefined if
     ptr was not returned by that function. The returned function
     may be passed back to installFunction() to reinstall it.

     To simplify certain use cases, if passed a falsy non-0 value
     (noting that 0 is a valid function table index), this function
     has no side effects and returns undefined.
  */
  target.uninstallFunction = function(ptr){
    if(!ptr && __NullPtr!==ptr) return undefined;

    const ft = target.functionTable();
    cache.freeFuncIndexes.push(ptr);
    const rc = ft.get(ptr);
    ft.set(ptr, null);
    return rc;
  };

  /**
     Given a WASM heap memory address and a data type name in the form
     (i8, i16, i32, i64, float (or f32), double (or f64)), this
     fetches the numeric value from that address and returns it as a
     number or, for the case of type='i64', a BigInt (with the caveat
     BigInt will trigger an exception if this.bigIntEnabled is
     falsy). Throws if given an invalid type.

     If the first argument is an array, it is treated as an array of
     addresses and the result is an array of the values from each of
     those address, using the same 2nd argument for determining the
     value type to fetch.

     As a special case, if type ends with a `*`, it is considered to
     be a pointer type and is treated as the WASM numeric type
     appropriate for the pointer size (==this.ptr.ir).

     While possibly not obvious, this routine and its poke()
     counterpart are how pointer-to-value _output_ parameters in
     WASM-compiled C code can be interacted with:

     ```
     const ptr = alloc(4);
     poke32(ptr, 0); // clear the ptr's value
     aCFuncWithOutputPtrToInt32Arg(ptr); // e.g. void foo(int *x);
     const result = peek32(ptr); // fetch ptr's value
     dealloc(ptr);
     ```

     scopedAlloc() and friends can be used to make handling of
     `ptr` safe against leaks in the case of an exception:

     ```
     let result;
     const scope = scopedAllocPush();
     try{
       const ptr = scopedAlloc(4);
       poke32(ptr, 0);
       aCFuncWithOutputPtrArg(ptr);
       result = peek32(ptr);
     }finally{
       scopedAllocPop(scope);
     }
     ```

     As a rule poke() must be called to set (typically zero out) the
     pointer's value, else it will contain an essentially random
     value.

     ACHTUNG: calling this often, e.g. in a loop, can have a noticably
     painful impact on performance. Rather than doing so, use
     heapForSize() to fetch the heap object and read directly from it.

     ACHTUNG #2: ptr may be a BigInt (and generally will be in 64-bit
     builds) but this function must coerce it into a Number in order
     to access the heap's contents. Ergo: BitInts outside of the
     (extrardinarily genereous) address range exposed to browser-side
     WASM may cause misbehavior.

     See also: poke()
  */
  target.peek = function f(ptr, type='i8'){
    if(type.endsWith('*')) type = __ptrIR;
    const c = (cache.memory && cache.heapSize === cache.memory.buffer.byteLength)
          ? cache : heapWrappers();
    const list = Array.isArray(ptr) ? [] : undefined;
    let rc;
    do{
      if(list) ptr = arguments[0].shift();
      switch(type){
          case 'i1':
          case 'i8': rc = c.HEAP8[Number(ptr/*tag:64bit*/)>>0]; break;
          case 'i16': rc = c.HEAP16[Number(ptr/*tag:64bit*/)>>1]; break;
          case 'i32': rc = c.HEAP32[Number(ptr/*tag:64bit*/)>>2]; break;
          case 'float': case 'f32': rc = c.HEAP32F[Number(ptr/*tag:64bit*/)>>2]; break;
          case 'double': case 'f64': rc = Number(c.HEAP64F[Number(ptr/*tag:64bit*/)>>3]); break;
          case 'i64':
            if(c.HEAP64){
              rc = __BigInt(c.HEAP64[Number(ptr/*tag:64bit*/)>>3]);
              break;
            }
            /* fallthru */
          default:
            toss('Invalid type for peek():',type);
      }
      if(list) list.push(rc);
    }while(list && arguments[0].length);
    return list || rc;
  };

  /**
     The counterpart of peek(), this sets a numeric value at the given
     WASM heap address, using the 3rd argument to define how many
     bytes are written. Throws if given an invalid type. See peek()
     for details about the `type` argument. If the 3rd argument ends
     with `*` then it is treated as a pointer type and this function
     behaves as if the 3rd argument were this.ptr.ir.

     If the first argument is an array, it is treated like a list
     of pointers and the given value is written to each one.

     Returns `this`. (Prior to 2022-12-09 it returned this function.)

     ACHTUNG #1: see peek()'s ACHTUNG #1.

     ACHTUNG #2: see peek()'s ACHTUNG #2.
  */
  target.poke = function(ptr, value, type='i8'){
    if (type.endsWith('*')) type = __ptrIR;
    const c = (cache.memory && cache.heapSize === cache.memory.buffer.byteLength)
          ? cache : heapWrappers();
    for(const p of (Array.isArray(ptr) ? ptr : [ptr])){
      switch (type) {
          case 'i1':
          case 'i8': c.HEAP8[Number(p/*tag:64bit*/)>>0] = value; continue;
          case 'i16': c.HEAP16[Number(p/*tag:64bit*/)>>1] = value; continue;
          case 'i32': c.HEAP32[Number(p/*tag:64bit*/)>>2] = value; continue;
          case 'float': case 'f32': c.HEAP32F[Number(p/*tag:64bit*/)>>2] = value; continue;
          case 'double': case 'f64': c.HEAP64F[Number(p/*tag:64bit*/)>>3] = value; continue;
          case 'i64':
            if(c.HEAP64){
              c.HEAP64[Number(p/*tag:64bit*/)>>3] = __BigInt(value);
              continue;
            }
            /* fallthru */
          default:
            toss('Invalid type for poke(): ' + type);
      }
    }
    return this;
  };

  /**
     Convenience form of peek() intended for fetching
     pointer-to-pointer values. If passed a single non-array argument
     it returns the value of that one pointer address. If passed
     multiple arguments, or a single array of arguments, it returns an
     array of their values.
  */
  target.peekPtr = (...ptr)=>target.peek( (1===ptr.length ? ptr[0] : ptr), __ptrIR );

  /**
     A variant of poke() intended for setting pointer-to-pointer
     values. Its differences from poke() are that (1) it defaults to a
     value of 0 and (2) it always writes to the pointer-sized heap
     view.
  */
  target.pokePtr = (ptr, value=0)=>target.poke(ptr, value, __ptrIR);

  /**
     Convenience form of peek() intended for fetching i8 values. If
     passed a single non-array argument it returns the value of that
     one pointer address. If passed multiple arguments, or a single
     array of arguments, it returns an array of their values.
  */
  target.peek8 = (...ptr)=>target.peek( (1===ptr.length ? ptr[0] : ptr), 'i8' );
  /**
     Convenience form of poke() intended for setting individual bytes.
     Its difference from poke() is that it always writes to the
     i8-sized heap view.
  */
  target.poke8 = (ptr, value)=>target.poke(ptr, value, 'i8');
  /** i16 variant of peek8(). */
  target.peek16 = (...ptr)=>target.peek( (1===ptr.length ? ptr[0] : ptr), 'i16' );
  /** i16 variant of poke8(). */
  target.poke16 = (ptr, value)=>target.poke(ptr, value, 'i16');
  /** i32 variant of peek8(). */
  target.peek32 = (...ptr)=>target.peek( (1===ptr.length ? ptr[0] : ptr), 'i32' );
  /** i32 variant of poke8(). */
  target.poke32 = (ptr, value)=>target.poke(ptr, value, 'i32');
  /** i64 variant of peek8(). Will throw if this build is not
      configured for BigInt support. */
  target.peek64 = (...ptr)=>target.peek( (1===ptr.length ? ptr[0] : ptr), 'i64' );
  /** i64 variant of poke8(). Will throw if this build is not
      configured for BigInt support. Note that this returns
      a BigInt-type value, not a Number-type value. */
  target.poke64 = (ptr, value)=>target.poke(ptr, value, 'i64');
  /** f32 variant of peek8(). */
  target.peek32f = (...ptr)=>target.peek( (1===ptr.length ? ptr[0] : ptr), 'f32' );
  /** f32 variant of poke8(). */
  target.poke32f = (ptr, value)=>target.poke(ptr, value, 'f32');
  /** f64 variant of peek8(). */
  target.peek64f = (...ptr)=>target.peek( (1===ptr.length ? ptr[0] : ptr), 'f64' );
  /** f64 variant of poke8(). */
  target.poke64f = (ptr, value)=>target.poke(ptr, value, 'f64');

  /** Deprecated alias for getMemValue() */
  target.getMemValue = target.peek;
  /** Deprecated alias for peekPtr() */
  target.getPtrValue = target.peekPtr;
  /** Deprecated alias for poke() */
  target.setMemValue = target.poke;
  /** Deprecated alias for pokePtr() */
  target.setPtrValue = target.pokePtr;

  /**
     Returns true if the given value appears to be legal for use as
     a WASM pointer value. Its _range_ of values is not (cannot be)
     validated except to ensure that it is a 32-bit integer with a
     value of 0 or greater. Likewise, it cannot verify whether the
     value actually refers to allocated memory in the WASM heap.

     Whether or not null or undefined are legal are context-dependent.
     They generally are legal but this function does not treat them as
     such because they're not strictly legal for passing as-is as WASM
     integer arguments.
  */
  target.isPtr32 = (ptr)=>(
    'number'===typeof ptr && ptr>=0 && ptr===(ptr|0)
  );

  /** 64-bit counterpart of isPtr32() and falls back to that function
      if ptr is not a BigInt. */
  target.isPtr64 = (ptr)=>(
    ('bigint'===typeof ptr) ? ptr >= 0 : target.isPtr32(ptr)
  );

  /**
     isPtr() is an alias for isPtr32() or isPtr64(), depending on the
     value of target.ptr.size.
  */
  target.isPtr = (4===__ptrSize) ? target.isPtr32 : target.isPtr64;

  /**
     Expects ptr to be a pointer into the WASM heap memory which
     refers to a NUL-terminated C-style string encoded as UTF-8.
     Returns the length, in bytes, of the string, as for `strlen(3)`.
     As a special case, if !ptr or if it's not a pointer then it
     returns `null`. Throws if ptr is out of range for
     target.heap8u().
  */
  target.cstrlen = function(ptr){
    if(!ptr || !target.isPtr/*64*/(ptr)) return null;
    ptr = Number(ptr) /*tag:64bit*/;
    const h = heapWrappers().HEAP8U;
    let pos = ptr;
    for( ; h[pos] !== 0; ++pos ){}
    return pos - ptr;
  };

  /** Internal helper to use in operations which need to distinguish
      between TypedArrays which are backed by a SharedArrayBuffer
      from those which are not. */
  const __SAB = ('undefined'===typeof SharedArrayBuffer)
        ? function(){/*dummy class*/} : SharedArrayBuffer;
  /** Returns true if the given TypedArray object is backed by a
      SharedArrayBuffer, else false. */
  const isSharedTypedArray = (aTypedArray)=>(aTypedArray.buffer instanceof __SAB);

  target.isSharedTypedArray = isSharedTypedArray;

  /**
     Returns either aTypedArray.slice(begin,end) (if
     aTypedArray.buffer is a SharedArrayBuffer) or
     aTypedArray.subarray(begin,end) (if it's not).

     This distinction is important for APIs which don't like to
     work on SABs, e.g. TextDecoder, and possibly for our
     own APIs which work on memory ranges which "might" be
     modified by other threads while they're working.

     begin and end may be of type Number or (in 64-bit builds) BigInt
     (which get coerced to Numbers).
  */
  const typedArrayPart = (aTypedArray, begin, end)=>{
    if( 8===__ptrSize ){
      // slice() and subarray() do not like BigInt args.
      if( 'bigint'===typeof begin ) begin = Number(begin);
      if( 'bigint'===typeof end ) end = Number(end);
    }
    return isSharedTypedArray(aTypedArray)
      ? aTypedArray.slice(begin, end)
      : aTypedArray.subarray(begin, end);
  };

  target.typedArrayPart = typedArrayPart;

  /**
     Uses TextDecoder to decode the given half-open range of the given
     TypedArray to a string. This differs from a simple call to
     TextDecoder in that it accounts for whether the first argument is
     backed by a SharedArrayBuffer or not, and can work more
     efficiently if it's not (TextDecoder refuses to act upon an SAB).

     If begin/end are not provided or are falsy then each defaults to
     the start/end of the array.
  */
  const typedArrayToString = (typedArray, begin, end)=>
        cache.utf8Decoder.decode(
          typedArrayPart(typedArray, begin, end)
        );

  target.typedArrayToString = typedArrayToString;

  /**
     Expects ptr to be a pointer into the WASM heap memory which
     refers to a NUL-terminated C-style string encoded as UTF-8. This
     function counts its byte length using cstrlen() then returns a
     JS-format string representing its contents. As a special case, if
     ptr is falsy or not a pointer, `null` is returned.
  */
  target.cstrToJs = function(ptr){
    const n = target.cstrlen(ptr);
    return n
      ? typedArrayToString(heapWrappers().HEAP8U, Number(ptr), Number(ptr)+n)
      : (null===n ? n : "");
  };

  /**
     Given a JS string, this function returns its UTF-8 length in
     bytes. Returns null if str is not a string.
  */
  target.jstrlen = function(str){
    /** Attribution: derived from Emscripten's lengthBytesUTF8() */
    if('string'!==typeof str) return null;
    const n = str.length;
    let len = 0;
    for(let i = 0; i < n; ++i){
      let u = str.charCodeAt(i);
      if(u>=0xd800 && u<=0xdfff){
        u = 0x10000 + ((u & 0x3FF) << 10) | (str.charCodeAt(++i) & 0x3FF);
      }
      if(u<=0x7f) ++len;
      else if(u<=0x7ff) len += 2;
      else if(u<=0xffff) len += 3;
      else len += 4;
    }
    return len;
  };

  /**
     Encodes the given JS string as UTF8 into the given TypedArray
     tgt, starting at the given offset and writing, at most, maxBytes
     bytes (including the NUL terminator if addNul is true, else no
     NUL is added). If it writes any bytes at all and addNul is true,
     it always NUL-terminates the output, even if doing so means that
     the NUL byte is all that it writes.

     If maxBytes is negative (the default) then it is treated as the
     remaining length of tgt, starting at the given offset.

     If writing the last character would surpass the maxBytes count
     because the character is multi-byte, that character will not be
     written (as opposed to writing a truncated multi-byte character).
     This can lead to it writing as many as 3 fewer bytes than
     maxBytes specifies.

     Returns the number of bytes written to the target, _including_
     the NUL terminator (if any). If it returns 0, it wrote nothing at
     all, which can happen if:

     - str is empty and addNul is false.
     - offset < 0.
     - maxBytes == 0.
     - maxBytes is less than the byte length of a multi-byte str[0].

     Throws if tgt is not an Int8Array or Uint8Array.

     Design notes:

     - In C's strcpy(), the destination pointer is the first
       argument. That is not the case here primarily because the 3rd+
       arguments are all referring to the destination, so it seems to
       make sense to have them grouped with it.

     - Emscripten's counterpart of this function (stringToUTF8Array())
       returns the number of bytes written sans NUL terminator. That
       is, however, ambiguous: str.length===0 or maxBytes===(0 or 1)
       all cause 0 to be returned.
  */
  target.jstrcpy = function(jstr, tgt, offset = 0, maxBytes = -1, addNul = true){
    /** Attribution: the encoding bits are taken from Emscripten's
        stringToUTF8Array(). */
    if(!tgt || (!(tgt instanceof Int8Array) && !(tgt instanceof Uint8Array))){
      toss("jstrcpy() target must be an Int8Array or Uint8Array.");
    }
    maxBytes = Number(maxBytes)/*tag:64bit*/;
    offset = Number(offset)/*tag:64bit*/;
    if(maxBytes<0) maxBytes = tgt.length - offset;
    if(!(maxBytes>0) || !(offset>=0)) return 0;
    let i = 0, max = jstr.length;
    const begin = offset, end = offset + maxBytes - (addNul ? 1 : 0);
    for(; i < max && offset < end; ++i){
      let u = jstr.charCodeAt(i);
      if(u>=0xd800 && u<=0xdfff){
        u = 0x10000 + ((u & 0x3FF) << 10) | (jstr.charCodeAt(++i) & 0x3FF);
      }
      if(u<=0x7f){
        if(offset >= end) break;
        tgt[offset++] = u;
      }else if(u<=0x7ff){
        if(offset + 1 >= end) break;
        tgt[offset++] = 0xC0 | (u >> 6);
        tgt[offset++] = 0x80 | (u & 0x3f);
      }else if(u<=0xffff){
        if(offset + 2 >= end) break;
        tgt[offset++] = 0xe0 | (u >> 12);
        tgt[offset++] = 0x80 | ((u >> 6) & 0x3f);
        tgt[offset++] = 0x80 | (u & 0x3f);
      }else{
        if(offset + 3 >= end) break;
        tgt[offset++] = 0xf0 | (u >> 18);
        tgt[offset++] = 0x80 | ((u >> 12) & 0x3f);
        tgt[offset++] = 0x80 | ((u >> 6) & 0x3f);
        tgt[offset++] = 0x80 | (u & 0x3f);
      }
    }
    if(addNul) tgt[offset++] = 0;
    return offset - begin;
  };

  /**
     Works similarly to C's strncpy(), copying, at most, n bytes (not
     characters) from srcPtr to tgtPtr. It copies until n bytes have
     been copied or a 0 byte is reached in src. _Unlike_ strncpy(), it
     returns the number of bytes it assigns in tgtPtr, _including_ the
     NUL byte (if any). If n is reached before a NUL byte in srcPtr,
     tgtPtr will _not_ be NULL-terminated. If a NUL byte is reached
     before n bytes are copied, tgtPtr will be NUL-terminated.

     If n is negative, cstrlen(srcPtr)+1 is used to calculate it, the
     +1 being for the NUL byte.

     Throws if tgtPtr or srcPtr are falsy. Results are undefined if:

     - either is not a pointer into the WASM heap or

     - srcPtr is not NUL-terminated AND n is less than srcPtr's
       logical length.

     ACHTUNG: it is possible to copy partial multi-byte characters
     this way, and converting such strings back to JS strings will
     have undefined results.
  */
  target.cstrncpy = function(tgtPtr, srcPtr, n){
    if(!tgtPtr || !srcPtr) toss("cstrncpy() does not accept NULL strings.");
    if(n<0) n = target.cstrlen(strPtr)+1;
    else if(!(n>0)) return 0;
    const heap = target.heap8u();
    let i = 0, ch;
    const tgtNumber = Number(tgtPtr), srcNumber = Number(srcPtr)/*tag:64bit*/;
    for(; i < n && (ch = heap[srcNumber+i]); ++i){
      heap[tgtNumber+i] = ch;
    }
    if(i<n) heap[tgtNumber + i++] = 0;
    return i;
  };

  /**
     For the given JS string, returns a Uint8Array of its contents
     encoded as UTF-8. If addNul is true, the returned array will have
     a trailing 0 entry, else it will not.
  */
  target.jstrToUintArray = (str, addNul=false)=>{
    return cache.utf8Encoder.encode(addNul ? (str+"\0") : str);
  };

  const __affirmAlloc = (obj,funcName)=>{
    if(!(obj.alloc instanceof Function) ||
       !(obj.dealloc instanceof Function)){
      toss("Object is missing alloc() and/or dealloc() function(s)",
           "required by",funcName+"().");
    }
  };

  const __allocCStr = function(jstr, returnWithLength, allocator, funcName){
    __affirmAlloc(target, funcName);
    if('string'!==typeof jstr) return null;
    const u = cache.utf8Encoder.encode(jstr),
          ptr = allocator(u.length+1);
    let toFree = ptr;
    try{
      const heap = heapWrappers().HEAP8U;
      heap.set(u, Number(ptr));
      heap[__ptrAdd(ptr, u.length)] = 0;
      toFree = __NullPtr;
      return returnWithLength ? [ptr, u.length] : ptr;
    }finally{
      if( toFree ) target.dealloc(toFree);
    }
  };

  /**
     Uses target.alloc() to allocate enough memory for jstrlen(jstr)+1
     bytes of memory, copies jstr to that memory using jstrcpy(),
     NUL-terminates it, and returns the pointer to that C-string.
     Ownership of the pointer is transfered to the caller, who must
     eventually pass the pointer to dealloc() to free it.

     If passed a truthy 2nd argument then its return semantics change:
     it returns [ptr,n], where ptr is the C-string's pointer and n is
     its cstrlen().

     Throws if `target.alloc` or `target.dealloc` are not functions.
  */
  target.allocCString =
    (jstr, returnWithLength=false)=>__allocCStr(jstr, returnWithLength,
                                                target.alloc, 'allocCString()');

  /**
     Starts an "allocation scope." All allocations made using
     scopedAlloc() are recorded in this scope and are freed when the
     value returned from this function is passed to
     scopedAllocPop().

     This family of functions requires that the API's object have both
     `alloc()` and `dealloc()` methods, else this function will throw.

     Intended usage:

     ```
     const scope = scopedAllocPush();
     try {
       const ptr1 = scopedAlloc(100);
       const ptr2 = scopedAlloc(200);
       const ptr3 = scopedAlloc(300);
       ...
       // Note that only allocations made via scopedAlloc()
       // are managed by this allocation scope.
     }finally{
       scopedAllocPop(scope);
     }
     ```

     The value returned by this function must be treated as opaque by
     the caller, suitable _only_ for passing to scopedAllocPop().
     Its type and value are not part of this function's API and may
     change in any given version of this code.

     `scopedAlloc.level` can be used to determine how many scoped
     alloc levels are currently active.
   */
  target.scopedAllocPush = function(){
    __affirmAlloc(target, 'scopedAllocPush');
    const a = [];
    cache.scopedAlloc.push(a);
    return a;
  };

  /**
     Cleans up all allocations made using scopedAlloc() in the context
     of the given opaque state object, which must be a value returned
     by scopedAllocPush(). See that function for an example of how to
     use this function. It also uninstalls any WASM functions
     installed with scopedInstallFunction().

     Though scoped allocations are managed like a stack, this API
     behaves properly if allocation scopes are popped in an order
     other than the order they were pushed. The intent is that it
     _always_ be used in a stack-like manner.

     If called with no arguments, it pops the most recent
     scopedAllocPush() result:

     ```
     scopedAllocPush();
     try{ ... } finally { scopedAllocPop(); }
     ```

     It's generally recommended that it be passed an explicit argument
     to help ensure that push/push are used in matching pairs, but in
     trivial code that may be a non-issue.
  */
  target.scopedAllocPop = function(state){
    __affirmAlloc(target, 'scopedAllocPop');
    const n = arguments.length
          ? cache.scopedAlloc.indexOf(state)
          : cache.scopedAlloc.length-1;
    if(n<0) toss("Invalid state object for scopedAllocPop().");
    if(0===arguments.length) state = cache.scopedAlloc[n];
    cache.scopedAlloc.splice(n,1);
    for(let p; (p = state.pop()); ){
      if(target.functionEntry(p)){
        //console.warn("scopedAllocPop() uninstalling function",p);
        target.uninstallFunction(p);
      }else{
        target.dealloc(p);
      }
    }
  };

  /**
     Allocates n bytes of memory using this.alloc() and records that
     fact in the state for the most recent call of scopedAllocPush().
     Ownership of the memory is given to scopedAllocPop(), which
     will clean it up when it is called. The memory _must not_ be
     passed to this.dealloc(). Throws if this API object is missing
     the required `alloc()` or `dealloc()` functions or no scoped
     alloc is active.

     See scopedAllocPush() for an example of how to use this function.

     The `level` property of this function can be queried to query how
     many scoped allocation levels are currently active.

     See also: scopedAllocPtr(), scopedAllocCString()
  */
  target.scopedAlloc = function(n){
    if(!cache.scopedAlloc.length){
      toss("No scopedAllocPush() scope is active.");
    }
    const p = __asPtrType(target.alloc(n));
    return cache.scopedAlloc.pushPtr(p);
  };

  Object.defineProperty(target.scopedAlloc, 'level', {
    configurable: false, enumerable: false,
    get: ()=>cache.scopedAlloc.length,
    set: ()=>toss("The 'active' property is read-only.")
  });

  /**
     Works identically to allocCString() except that it allocates the
     memory using scopedAlloc().

     Will throw if no scopedAllocPush() call is active.
  */
  target.scopedAllocCString =
    (jstr, returnWithLength=false)=>__allocCStr(jstr, returnWithLength,
                                                target.scopedAlloc,
                                                'scopedAllocCString()');

  // impl for allocMainArgv() and scopedAllocMainArgv().
  const __allocMainArgv = function(isScoped, list){
    const pList = target[
      isScoped ? 'scopedAlloc' : 'alloc'
    ]((list.length + 1) * target.ptr.size);
    let i = 0;
    list.forEach((e)=>{
      target.pokePtr(__ptrAdd(pList, target.ptr.size * i++),
                     target[
                       isScoped ? 'scopedAllocCString' : 'allocCString'
                     ](""+e));
    });
    target.pokePtr(__ptrAdd(pList, target.ptr.size * i), 0);
    return pList;
  };

  /**
     Creates an array, using scopedAlloc(), suitable for passing to a
     C-level main() routine. The input is a collection with a length
     property and a forEach() method. A block of memory
     (list.length+1) entries long is allocated and each pointer-sized
     block of that memory is populated with a scopedAllocCString()
     conversion of the (""+value) of each element, with the exception
     that the final entry is a NULL pointer. Returns a pointer to the
     start of the list, suitable for passing as the 2nd argument to a
     C-style main() function.

     Throws if scopedAllocPush() is not active.

     Design note: the returned array is allocated with an extra NULL
     pointer entry to accommodate certain APIs, but client code which
     does not need that functionality should treat the returned array
     as list.length entries long.
  */
  target.scopedAllocMainArgv = (list)=>__allocMainArgv(true, list);

  /**
     Identical to scopedAllocMainArgv() but uses alloc() instead of
     scopedAlloc().
  */
  target.allocMainArgv = (list)=>__allocMainArgv(false, list);

  /**
     Expects to be given a C-style string array and its length. It
     returns a JS array of strings and/or nulls: any entry in the
     pArgv array which is NULL results in a null entry in the result
     array. If argc is 0 then an empty array is returned.

     Results are undefined if any entry in the first argc entries of
     pArgv are neither 0 (NULL) nor legal UTF-format C strings.

     To be clear, the expected C-style arguments to be passed to this
     function are `(int, char **)` (optionally const-qualified).
  */
  target.cArgvToJs = (argc, pArgv)=>{
    const list = [];
    for(let i = 0; i < argc; ++i){
      const arg = target.peekPtr(__ptrAdd(pArgv, target.ptr.size * i));
      list.push( arg ? target.cstrToJs(arg) : null );
    }
    return list;
  };

  /**
     Wraps function call func() in a scopedAllocPush() and
     scopedAllocPop() block, such that all calls to scopedAlloc() and
     friends from within that call will have their memory freed
     automatically when func() returns. If func throws or propagates
     an exception, the scope is still popped, otherwise it returns the
     result of calling func().
  */
  target.scopedAllocCall = function(func){
    target.scopedAllocPush();
    try{ return func() } finally{ target.scopedAllocPop() }
  };

  /** Internal impl for allocPtr() and scopedAllocPtr(). */
  const __allocPtr = function(howMany, safePtrSize, method){
    __affirmAlloc(target, method);
    const pIr = safePtrSize ? 'i64' : __ptrIR;
    let m = target[method](howMany * (safePtrSize ? 8 : __ptrSize));
    target.poke(m, 0, pIr)
    if(1===howMany){
      return m;
    }
    const a = [m];
    for(let i = 1; i < howMany; ++i){
      m = __ptrAdd(m, (safePtrSize ? 8 : __ptrSize));
      a[i] = m;
      target.poke(m, 0, pIr);
    }
    return a;
  };

  /**
     Allocates one or more pointers as a single chunk of memory and
     zeroes them out.

     The first argument is the number of pointers to allocate. The
     second specifies whether they should use a "safe" pointer size (8
     bytes) or whether they may use the default pointer size
     (typically 4 but also possibly 8).

     How the result is returned depends on its first argument: if
     passed 1, it returns the allocated memory address. If passed more
     than one then an array of pointer addresses is returned, which
     can optionally be used with "destructuring assignment" like this:

     ```
     const [p1, p2, p3] = allocPtr(3);
     ```

     ACHTUNG: when freeing the memory, pass only the _first_ result
     value to dealloc(). The others are part of the same memory chunk
     and must not be freed separately.

     The reason for the 2nd argument is...

     When one of the returned pointers will refer to a 64-bit value,
     e.g. a double or int64, and that value must be written or fetched,
     e.g. using poke() or peek(), it is important that
     the pointer in question be aligned to an 8-byte boundary or else
     it will not be fetched or written properly and will corrupt or
     read neighboring memory. It is only safe to pass false when the
     client code is certain that it will only get/fetch 4-byte values
     (or smaller).
  */
  target.allocPtr =
    (howMany=1, safePtrSize=true)=>__allocPtr(howMany, safePtrSize, 'alloc');

  /**
     Identical to allocPtr() except that it allocates using scopedAlloc()
     instead of alloc().
  */
  target.scopedAllocPtr =
    (howMany=1, safePtrSize=true)=>__allocPtr(howMany, safePtrSize, 'scopedAlloc');

  /**
     If target.exports[name] exists, it is returned, else an
     exception is thrown.
  */
  target.xGet = function(name){
    return target.exports[name] || toss("Cannot find exported symbol:",name);
  };

  const __argcMismatch =
        (f,n)=>toss(f+"() requires",n,"argument(s).");

  /**
     Looks up a WASM-exported function named fname from
     target.exports. If found, it is called, passed all remaining
     arguments, and its return value is returned to xCall's caller. If
     not found, an exception is thrown. This function does no
     conversion of argument or return types, but see xWrap() and
     xCallWrapped() for variants which do.

     If the first argument is a function is is assumed to be
     a WASM-bound function and is used as-is instead of looking up
     the function via xGet().

     As a special case, if passed only 1 argument after the name and
     that argument in an Array, that array's entries become the
     function arguments. (This is not an ambiguous case because it's
     not legal to pass an Array object to a WASM function.)
  */
  target.xCall = function(fname, ...args){
    const f = (fname instanceof Function) ? fname : target.xGet(fname);
    if(!(f instanceof Function)) toss("Exported symbol",fname,"is not a function.");
    if(f.length!==args.length){
      __argcMismatch(((f===fname) ? f.name : fname),f.length)
      /* This is arguably over-pedantic but we want to help clients keep
         from shooting themselves in the foot when calling C APIs. */;
    }
    return (2===arguments.length && Array.isArray(arguments[1]))
      ? f.apply(null, arguments[1])
      : f.apply(null, args);
  };

  /**
     State for use with xWrap().
  */
  cache.xWrap = Object.create(null);
  cache.xWrap.convert = Object.create(null);
  /** Map of type names to argument conversion functions. */
  cache.xWrap.convert.arg = new Map;
  /** Map of type names to return result conversion functions. */
  cache.xWrap.convert.result = new Map;
  /** Scope-local convenience aliases. */
  const xArg = cache.xWrap.convert.arg, xResult = cache.xWrap.convert.result;

  const __xArgPtr = __asPtrType;
  xArg
    .set('i64', __BigInt)
    .set('i32', (i)=>i|0)
    .set('i16', (i)=>((i | 0) & 0xFFFF))
    .set('i8', (i)=>((i | 0) & 0xFF))
    .set('f32', (i)=>Number(i).valueOf())
    .set('float', xArg.get('f32'))
    .set('f64', xArg.get('f32'))
    .set('double', xArg.get('f64'))
    .set('int', xArg.get('i32'))
    .set('null', (i)=>i)
    .set(null, xArg.get('null'))
    .set('**', __xArgPtr)
    .set('*', __xArgPtr)
  ;
  xResult.set('*', __xArgPtr)
    .set('pointer', __xArgPtr)
    .set('number', (v)=>Number(v))
    .set('void', (v)=>undefined)
    .set(undefined, xResult.get('void'))
    .set('null', (v)=>v)
    .set(null, xResult.get('null'))
  ;

  /* Copy xArg[...] handlers to xResult[...] for cases which have
     identical semantics. Also add pointer-style variants of those
     xArg entries to both xArg and xResult. */
  for(const t of [
    'i8', 'i16', 'i32', 'i64', 'int',
    'f32', 'float', 'f64', 'double'
  ]){
    xArg.set(t+'*', __xArgPtr);
    xResult.set(t+'*', __xArgPtr);
    xResult.set(
      t, xArg.get(t)
        || toss("Maintenance required: missing arg converter for",t)
    );
  }

  /**
     In order for args of type string to work in various contexts in
     the sqlite3 API, we need to pass them on as, variably, a C-string
     or a pointer value. Thus for ARGs of type 'string' and
     '*'/'pointer' we behave differently depending on whether the
     argument is a string or not:

     - If v is a string, scopeAlloc() a new C-string from it and return
       that temp string's pointer.

     - Else return the value from the arg adapter defined for
       target.ptr.ir.

     TODO? Permit an Int8Array/Uint8Array and convert it to a string?
     Would that be too much magic concentrated in one place, ready to
     backfire? We handle that at the client level in sqlite3 with a
     custom argument converter.
  */
  const __xArgString = (v)=>{
    return ('string'===typeof v)
      ? target.scopedAllocCString(v)
      : __asPtrType(v);
  };

  xArg.set('string', __xArgString)
    .set('utf8', __xArgString)
    // (much later: why did we do this?) .set('pointer', __xArgString)
  ;

  xResult
    .set('string', (i)=>target.cstrToJs(i))
    .set('utf8', xResult.get('string'))
    .set('string:dealloc', (i)=>{
      try { return i ? target.cstrToJs(i) : null }
      finally{ target.dealloc(i) }
    })
    .set('utf8:dealloc', xResult.get('string:dealloc'))
    .set('json', (i)=>JSON.parse(target.cstrToJs(i)))
    .set('json:dealloc', (i)=>{
      try{ return i ? JSON.parse(target.cstrToJs(i)) : null }
      finally{ target.dealloc(i) }
    });

  /**
     Internal-use-only base class for FuncPtrAdapter and potentially
     additional stateful argument adapter classes.

     Its main interface (convertArg()) is strictly internal, not to be
     exposed to client code, as it may still need re-shaping. Only the
     constructors of concrete subclasses should be exposed to clients,
     and those in such a way that does not hinder internal redesign of
     the convertArg() interface.
  */
  const AbstractArgAdapter = class {
    constructor(opt){
      this.name = opt.name || 'unnamed adapter';
    }
    /**
       Gets called via xWrap() to "convert" v to whatever type
       this specific class supports.

       argIndex is the argv index of _this_ argument in the
       being-xWrap()'d call. argv is the current argument list
       undergoing xWrap() argument conversion. argv entries to the
       left of argIndex will have already undergone transformation and
       those to the right will not have (they will have the values the
       client-level code passed in, awaiting conversion). The RHS
       indexes must never be relied upon for anything because their
       types are indeterminate, whereas the LHS values will be
       WASM-compatible values by the time this is called.

       The reason for the argv and argIndex arguments is that we
       frequently need more context than v for a specific conversion,
       and that context invariably lies in the LHS arguments of v.
       Examples of how this is useful can be found in FuncPtrAdapter.
    */
    convertArg(v,argv,argIndex){
      toss("AbstractArgAdapter must be subclassed.");
    }
  };

  /**
     This type is recognized by xWrap() as a proxy for converting a JS
     function to a C-side function, either permanently, for the
     duration of a single call into the C layer, or semi-contextual,
     where it may keep track of a single binding for a given context
     and uninstall the binding if it's replaced.

     The constructor requires an options object with these properties:

     - name (optional): string describing the function binding. This
       is solely for debugging and error-reporting purposes. If not
       provided, an empty string is assumed.

     - signature: a function signature string compatible with
       jsFuncToWasm().

     - bindScope (string): one of ('transient', 'context',
       'singleton', 'permanent'). Bind scopes are:

       - 'transient': it will convert JS functions to WASM only for
         the duration of the xWrap()'d function call, using
         scopedInstallFunction(). Before that call returns, the
         WASM-side binding will be uninstalled.

       - 'singleton': holds one function-pointer binding for this
         instance. If it's called with a different function pointer,
         it uninstalls the previous one after converting the new
         value. This is only useful for use with "global" functions
         which do not rely on any state other than this function
         pointer. If the being-converted function pointer is intended
         to be mapped to some sort of state object (e.g. an
         `sqlite3*`) then "context" (see below) is the proper mode.

       - 'context': similar to singleton mode but for a given
         "context", where the context is a key provided by the user
         and possibly dependent on a small amount of call-time
         context. This mode is the default if bindScope is _not_ set
         but a property named contextKey (described below) is.

       - 'permanent': the function is installed and left there
         forever. There is no way to recover its pointer address
         later on for cleanup purposes. i.e. it effectively leaks.

     - callProxy (function): if set, this must be a function which
       will act as a proxy for any "converted" JS function. It is
       passed the being-converted function value and must return
       either that function or a function which acts on its
       behalf. The returned function will be the one which gets
       installed into the WASM function table. The proxy must perform
       any required argument conversion (it will be called from C
       code, so will receive C-format arguments) before passing them
       on to the being-converted function. Whether or not the proxy
       itself must return a value depends on the context. If it does,
       it must be a WASM-friendly value, as it will be returning from
       a call made from WASM code.

     - contextKey (function): is only used if bindScope is 'context'
       or if bindScope is not set and this function is, in which case
       a bindScope of 'context' is assumed. This function gets bound
       to this object, so its "this" is this object. It gets passed
       (argv,argIndex), where argIndex is the index of _this_ function
       in its _wrapping_ function's arguments, and argv is the
       _current_ still-being-xWrap()-processed args array. (Got all
       that?) When thisFunc(argv,argIndex) is called by xWrap(), all
       arguments in argv to the left of argIndex will have been
       processed by xWrap() by the time this is called. argv[argIndex]
       will be the value the user passed in to the xWrap()'d function
       for the argument this FuncPtrAdapter is mapped to. Arguments to
       the right of argv[argIndex] will not yet have been converted
       before this is called. The function must return a key which
       uniquely identifies this function mapping context for _this_
       FuncPtrAdapter instance (other instances are not considered),
       taking into account that C functions often take some sort of
       state object as one or more of their arguments. As an example,
       if the xWrap()'d function takes `(int,T*,functionPtr,X*)` then
       this FuncPtrAdapter instance is argv[2], and contextKey(argv,2)
       might return 'T@'+argv[1], or even just argv[1].  Note,
       however, that the (`X*`) argument will not yet have been
       processed by the time this is called and should not be used as
       part of that key because its pre-conversion data type might be
       unpredictable. Similarly, care must be taken with C-string-type
       arguments: those to the left in argv will, when this is called,
       be WASM pointers, whereas those to the right might (and likely
       do) have another data type. When using C-strings in keys, never
       use their pointers in the key because most C-strings in this
       constellation are transient. Conversely, the pointer address
       makes an ideal key for longer-lived native pointer types.

     Yes, that ^^^ is quite awkward, but it's what we have. In
     context, as it were, it actually makes some sense, but one must
     look under its hook a bit to understand why it's shaped the
     way it is.

     The constructor only saves the above state for later, and does
     not actually bind any functions. The conversion, if any, is
     performed when its convertArg() method is called via xWrap().

     Shortcomings:

     - These "reverse" bindings, i.e. calling into a JS-defined
       function from a WASM-defined function (the generated proxy
       wrapper), lack all type conversion support. That means, for
       example, that...

     - Function pointers which include C-string arguments may still
       need a level of hand-written wrappers around them, depending on
       how they're used, in order to provide the client with JS
       strings. Alternately, clients will need to perform such
       conversions on their own, e.g. using cstrToJs(). The purpose of
       the callProxy() method is to account for such cases.
  */
  xArg.FuncPtrAdapter = class FuncPtrAdapter extends AbstractArgAdapter {
    constructor(opt) {
      super(opt);
      if(xArg.FuncPtrAdapter.warnOnUse){
        console.warn('xArg.FuncPtrAdapter is an internal-only API',
                     'and is not intended to be invoked from',
                     'client-level code. Invoked with:',opt);
      }
      this.name = opt.name || "unnamed";
      this.signature = opt.signature;
      if(opt.contextKey instanceof Function){
        this.contextKey = opt.contextKey;
        if(!opt.bindScope) opt.bindScope = 'context';
      }
      this.bindScope = opt.bindScope
        || toss("FuncPtrAdapter options requires a bindScope (explicit or implied).");
      if(FuncPtrAdapter.bindScopes.indexOf(opt.bindScope)<0){
        toss("Invalid options.bindScope ("+opt.bindMod+") for FuncPtrAdapter. "+
             "Expecting one of: ("+FuncPtrAdapter.bindScopes.join(', ')+')');
      }
      this.isTransient = 'transient'===this.bindScope;
      this.isContext = 'context'===this.bindScope;
      this.isPermanent = 'permanent'===this.bindScope;
      this.singleton = ('singleton'===this.bindScope) ? [] : undefined;
      //console.warn("FuncPtrAdapter()",opt,this);
      this.callProxy = (opt.callProxy instanceof Function)
        ? opt.callProxy : undefined;
    }

    /**
       The static class members are defined outside of the class to
       work around an emcc toolchain build problem: one of the tools
       in emsdk v3.1.42 does not support the static keyword.
    */

    /* Dummy impl. Overwritten per-instance as needed. */
    contextKey(argv,argIndex){
      return this;
    }

    /**
       Returns this object's mapping for the given context key, in the
       form of an an array, creating the mapping if needed. The key
       may be anything suitable for use in a Map.

       The returned array is intended to be used as a pair of
       [JSValue, WasmFuncPtr], where the first element is one passed
       to this.convertArg() and the second is its WASM form.
    */
    contextMap(key){
      const cm = (this.__cmap || (this.__cmap = new Map));
      let rc = cm.get(key);
      if(undefined===rc) cm.set(key, (rc = []));
      return rc;
    }

    /**
       Gets called via xWrap() to "convert" v to a WASM-bound function
       pointer. If v is one of (a WASM pointer, null, undefined) then
       (v||0) is returned and any earlier function installed by this
       mapping _might_, depending on how it's bound, be uninstalled.
       If v is not one of those types, it must be a Function, for
       which this method creates (if needed) a WASM function binding
       and returns the WASM pointer to that binding.

       If this instance is not in 'transient' mode, it will remember
       the binding for at least the next call, to avoid recreating the
       function binding unnecessarily.

       If it's passed a pointer(ish) value for v, it assumes it's a
       WASM function pointer and does _not_ perform any function
       binding, so this object's bindMode is irrelevant/ignored for
       such cases.

       See the parent class's convertArg() docs for details on what
       exactly the 2nd and 3rd arguments are.
    */
    convertArg(v,argv,argIndex){
      let pair = this.singleton;
      if(!pair && this.isContext){
        pair = this.contextMap(this.contextKey(argv,argIndex));
        //FuncPtrAdapter.debugOut(this.name, this.signature, "contextKey() =",this.contextKey(argv,argIndex), pair);
      }
      if( 0 ){
        FuncPtrAdapter.debugOut("FuncPtrAdapter.convertArg()",this.name,
                                'signature =',this.signature,
                                'transient ?=',this.transient,
                                'pair =',pair,
                                'v =',v);
      }
      if(pair && 2===pair.length && pair[0]===v){
        /* We have already handled this function. */
        return pair[1];
      }
      if(v instanceof Function){
        /* Install a WASM binding and return its pointer. */
        //FuncPtrAdapter.debugOut("FuncPtrAdapter.convertArg()",this.name,this.signature,this.transient,v,pair);
        if(this.callProxy){
          v = this.callProxy(v);
        }
        const fp = __installFunction(v, this.signature, this.isTransient);
        if(FuncPtrAdapter.debugFuncInstall){
          FuncPtrAdapter.debugOut("FuncPtrAdapter installed", this,
                                  this.contextKey(argv,argIndex), '@'+fp, v);
        }
        if(pair){
          /* Replace existing stashed mapping */
          if(pair[1]){
            if(FuncPtrAdapter.debugFuncInstall){
              FuncPtrAdapter.debugOut("FuncPtrAdapter uninstalling", this,
                                      this.contextKey(argv,argIndex), '@'+pair[1], v);
            }
            try{
              /* Because the pending native call might rely on the
                 pointer we're replacing, e.g. as is normally the case
                 with sqlite3's xDestroy() methods, we don't
                 immediately uninstall but instead add its pointer to
                 the scopedAlloc stack, which will be cleared when the
                 xWrap() mechanism is done calling the native
                 function. We're relying very much here on xWrap()
                 having pushed an alloc scope.
              */
              cache.scopedAlloc.pushPtr(pair[1]);
            }
            catch(e){/*ignored*/}
          }
          pair[0] = arguments[0] || __NullPtr/*the original v*/;
          pair[1] = fp;
        }
        return fp;
      }else if(target.isPtr(v) || null===v || undefined===v){
        if(pair && pair[1] && pair[1]!==v){
          /* uninstall stashed mapping and replace stashed mapping with v. */
          if(FuncPtrAdapter.debugFuncInstall){
            FuncPtrAdapter.debugOut("FuncPtrAdapter uninstalling", this,
                                    this.contextKey(argv,argIndex), '@'+pair[1], v);
          }
          try{cache.scopedAlloc.pushPtr(pair[1]);/*see notes above*/}
          catch(e){/*ignored*/}
          pair[0] = pair[1] = (v || __NullPtr);
        }
        return v || __NullPtr;
      }else{
        throw new TypeError("Invalid FuncPtrAdapter argument type. "+
                            "Expecting a function pointer or a "+
                            (this.name ? this.name+' ' : '')+
                            "function matching signature "+
                            this.signature+".");
      }
    }/*convertArg()*/
  }/*FuncPtrAdapter*/;

  /** If true, the constructor emits a warning. The intent is that
      this be set to true after bootstrapping of the higher-level
      client library is complete, to warn downstream clients that
      they shouldn't be relying on this implementation detail which
      does not have a stable interface. */
  xArg.FuncPtrAdapter.warnOnUse = false;

  /** If true, convertArg() will call FuncPtrAdapter.debugOut() when
      it (un)installs a function binding to/from WASM. Note that
      deinstallation of bindScope=transient bindings happens via
      scopedAllocPop() so will not be output. */
  xArg.FuncPtrAdapter.debugFuncInstall = false;

  /** Function used for debug output. */
  xArg.FuncPtrAdapter.debugOut = console.debug.bind(console);

  /**
     List of legal values for the FuncPtrAdapter bindScope config
     option.
  */
  xArg.FuncPtrAdapter.bindScopes = [
    'transient', 'context', 'singleton', 'permanent'
  ];

  /** Throws if xArg.get(t) returns falsy. */
  const __xArgAdapterCheck =
        (t)=>xArg.get(t) || toss("Argument adapter not found:",t);

  /** Throws if xResult.get(t) returns falsy. */
  const __xResultAdapterCheck =
        (t)=>xResult.get(t) || toss("Result adapter not found:",t);

  /**
     Fetches the xWrap() argument adapter mapped to t, calls it,
     passing in all remaining arguments, and returns the result.
     Throws if t is not mapped to an argument converter.
  */
  cache.xWrap.convertArg = (t,...args)=>__xArgAdapterCheck(t)(...args);
  /**
     Identical to convertArg() except that it does not perform
     an is-defined check on the mapping to t before invoking it.
  */
  cache.xWrap.convertArgNoCheck = (t,...args)=>xArg.get(t)(...args);

  /**
     Fetches the xWrap() result adapter mapped to t, calls it, passing
     it v, and returns the result.  Throws if t is not mapped to an
     argument converter.
  */
  cache.xWrap.convertResult =
    (t,v)=>(null===t ? v : (t ? __xResultAdapterCheck(t)(v) : undefined));
  /**
     Identical to convertResult() except that it does not perform an
     is-defined check on the mapping to t before invoking it.
  */
  cache.xWrap.convertResultNoCheck =
    (t,v)=>(null===t ? v : (t ? xResult.get(t)(v) : undefined));

  /**
     Creates a wrapper for another function which converts the arguments
     of the wrapper to argument types accepted by the wrapped function,
     then converts the wrapped function's result to another form
     for the wrapper.

     The first argument must be one of:

     - A JavaScript function.
     - The name of a WASM-exported function. xGet() is used to fetch
       the exported function, which throws if it's not found.
     - A pointer into the indirect function table. e.g. a pointer
       returned from target.installFunction().

     It returns either the passed-in function or a wrapper for that
     function which converts the JS-side argument types into WASM-side
     types and converts the result type.

     The second argument, `resultType`, describes the conversion for
     the wrapped functions result. A literal `null` or the string
     `'null'` both mean to return the original function's value as-is
     (mnemonic: there is "null" conversion going on). Literal
     `undefined` or the string `"void"` both mean to ignore the
     function's result and return `undefined`. Aside from those two
     special cases, the `resultType` value may be one of the values
     described below or any mapping installed by the client using
     xWrap.resultAdapter().

     If passed 3 arguments and the final one is an array, that array
     must contain a list of type names (see below) for adapting the
     arguments from JS to WASM.  If passed 2 arguments, more than 3,
     or the 3rd is not an array, all arguments after the 2nd (if any)
     are treated as type names. i.e.:

     ```
     xWrap('funcname', 'i32', 'string', 'f64');
     // is equivalent to:
     xWrap('funcname', 'i32', ['string', 'f64']);
     ```

     This function enforces that the given list of arguments has the
     same arity as the being-wrapped function (as defined by its
     `length` property) and it will throw if that is not the case.
     Similarly, the created wrapper will throw if passed a differing
     argument count. The intent of that strictness is to help catch
     coding errors in using JS-bound WASM functions earlier rather
     than laer.

     Type names are symbolic names which map the arguments to an
     adapter function to convert, if needed, the value before passing
     it on to WASM or to convert a return result from WASM. The list
     of pre-defined names:

     - `i8`, `i16`, `i32` (args and results): all integer conversions
       which convert their argument to an integer and truncate it to
       the given bit length.

     - `*`, `**`, and `pointer` (args): are assumed to be WASM pointer
       values and are returned coerced to an appropriately-sized
       pointer value (i32 or i64). Non-numeric values will coerce to 0
       and out-of-range values will have undefined results (just as
       with any pointer misuse).

     - `*` and `pointer` (results): aliases for the current
       WASM pointer numeric type.

     - `**` (args): is simply a descriptive alias for the WASM pointer
       type. It's primarily intended to mark output-pointer arguments,
       noting that JS's view of WASM does not distinguish between
       pointers and pointers-to-pointers, so all such interpretation
       of `**`, as distinct from `*`, necessarily happens at the
       client level.

     - `NumType*` (args): a type name in this form, where T is
       the name of a numeric mapping, e.g. 'int16' or 'double',
       is treated like `*`.

     - `i64` (args and results): passes the value to BigInt() to
       convert it to an int64. This conversion will if bigIntEnabled
       is falsy.

     - `f32` (`float`), `f64` (`double`) (args and results): pass
       their argument to Number(). i.e. the adapter does not currently
       distinguish between the two types of floating-point numbers.

     - `number` (results): converts the result to a JS Number using
       Number(theValue). This is for result conversions only, as it's
       not possible to generically know which type of number to
       convert arguments to.

     Non-numeric conversions include:

     - `null` literal or `"null"` string (args and results): perform
       no translation and pass the arg on as-is. This is primarily
       useful for results but may have a use or two for arguments.

     - `string` or `utf8` (args): has two different semantics in order
       to accommodate various uses of certain C APIs
       (e.g. output-style strings)...

       - If the arg is a JS string, it creates a _temporary_
         UTF-8-encoded C-string to pass to the exported function,
         cleaning it up before the wrapper returns. If a long-lived
         C-string pointer is required, that requires client-side code
         to create the string then pass its pointer to the function.

       - Else the arg is assumed to be a pointer to a string the
         client has already allocated and it's passed on as
         a WASM pointer.

     - `string` or `utf8` (results): treats the result value as a
       const C-string, encoded as UTF-8, copies it to a JS string,
       and returns that JS string.

     - `string:dealloc` or `utf8:dealloc` (results): treats the result
       value as a non-const UTF-8 C-string, ownership of which has
       just been transfered to the caller. It copies the C-string to a
       JS string, frees the C-string using dealloc(), and returns the
       JS string. If such a result value is NULL, the JS result is
       `null`. Achtung: when using an API which returns results from a
       specific allocator, e.g. `my_malloc()`, this conversion _is not
       legal_. Instead, an equivalent conversion which uses the
       appropriate deallocator is required. For example:

```js
   target.xWrap.resultAdapter('string:my_free',(i)=>{
     try { return i ? target.cstrToJs(i) : null; }
     finally{ target.exports.my_free(i); }
   };
```

     - `json` (results): treats the result as a const C-string and
       returns the result of passing the converted-to-JS string to
       JSON.parse(). Returns `null` if the C-string is a NULL pointer.

     - `json:dealloc` (results): works exactly like `string:dealloc` but
       returns the same thing as the `json` adapter. Note the
       warning in `string:dealloc` regarding matching allocators and
       deallocators.

     The type names for results and arguments are validated when
     xWrap() is called and any unknown names will trigger an
     exception.

     Clients may map their own result and argument adapters using
     xWrap.resultAdapter() and xWrap.argAdapter(), noting that not all
     type conversions are valid for both arguments _and_ result types
     as they often have different memory ownership requirements.

     Design note: the ability to pass in a JS function as the first
     argument is of relatively limited use, primarily for testing
     argument and result converters. JS functions, by and large, will
     not want to deal with C-type arguments.

     TODOs:

     - Figure out how/whether we can (semi-)transparently handle
       pointer-type _output_ arguments. Those currently require
       explicit handling by allocating pointers, assigning them before
       the call using poke(), and fetching them with
       peek() after the call. We may be able to automate some
       or all of that.

     - Figure out whether it makes sense to extend the arg adapter
       interface such that each arg adapter gets an array containing
       the results of the previous arguments in the current call. That
       might allow some interesting type-conversion feature. Use case:
       handling of the final argument to sqlite3_prepare_v2() depends
       on the type (pointer vs JS string) of its 2nd
       argument. Currently that distinction requires hand-writing a
       wrapper for that function. That case is unusual enough that
       abstracting it into this API (and taking on the associated
       costs) may well not make good sense.
  */
  target.xWrap = function callee(fArg, resultType, ...argTypes){
    if(3===arguments.length && Array.isArray(arguments[2])){
      argTypes = arguments[2];
    }
    if(target.isPtr(fArg)){
      fArg = target.functionEntry(fArg)
        || toss("Function pointer not found in WASM function table.");
    }
    const fIsFunc = (fArg instanceof Function);
    const xf = fIsFunc ? fArg : target.xGet(fArg);
    if(fIsFunc) fArg = xf.name || 'unnamed function';
    if(argTypes.length!==xf.length) __argcMismatch(fArg, xf.length);
    if( 0===xf.length
        && (null===resultType || 'null'===resultType) ){
      /* Func taking no args with an as-is return. We don't need a wrapper. */
      return xf;
    }
    /*Verify the arg type conversions are valid...*/;
    __xResultAdapterCheck(resultType);
    for(const t of argTypes){
      if(t instanceof AbstractArgAdapter) xArg.set(t, (...args)=>t.convertArg(...args));
      else __xArgAdapterCheck(t);
    }
    const cxw = cache.xWrap;
    if(0===xf.length){
      // No args to convert, so we can create a simpler wrapper...
      return (...args)=>(args.length
                         ? __argcMismatch(fArg, xf.length)
                         : cxw.convertResult(resultType, xf.call(null)));
    }
    return function(...args){
      if(args.length!==xf.length) __argcMismatch(fArg, xf.length);
      const scope = target.scopedAllocPush();
      try{
        /*
          Maintenance reminder re. arguments passed to convertArg():
          The public interface of argument adapters is that they take
          ONE argument and return a (possibly) converted result for
          it. The passing-on of arguments after the first is an
          internal implementation detail for the sake of
          AbstractArgAdapter, and not to be relied on or documented
          for other cases. The fact that this is how
          AbstractArgAdapter.convertArgs() gets its 2nd+ arguments,
          and how FuncPtrAdapter.contextKey() gets its args, is also
          an implementation detail and subject to change. i.e. the
          public interface of 1 argument is stable.  The fact that any
          arguments may be passed in after that one, and what those
          arguments are, is _not_ part of the public interface and is
          _not_ stable.

          Maintenance reminder: the Ember framework modifies the core
          Array type, breaking for-in loops:

          https://sqlite.org/forum/forumpost/b549992634b55104
        */
        let i = 0;
        if( callee.debug ){
          console.debug("xWrap() preparing: resultType ",resultType, 'xf',xf,"argTypes",argTypes,"args",args);
        }
        for(; i < args.length; ++i) args[i] = cxw.convertArgNoCheck(
          argTypes[i], args[i], args, i
        );
        if( callee.debug ){
          console.debug("xWrap() calling: resultType ",resultType, 'xf',xf,"argTypes",argTypes,"args",args);
        }
        return cxw.convertResultNoCheck(resultType, xf.apply(null,args));
      }finally{
        target.scopedAllocPop(scope);
      }
    };
  }/*xWrap()*/;

  /**
     Internal impl for xWrap.resultAdapter() and argAdapter().

     func = one of xWrap.resultAdapter or xWrap.argAdapter.

     argc = the number of args in the wrapping call to this
     function.

     typeName = the first arg to the wrapping function.

     adapter = the second arg to the wrapping function.

     modeName = a descriptive name of the wrapping function for
     error-reporting purposes.

     xcvPart = one of xResult or xArg.

     This acts as either a getter (if 1===argc) or setter (if
     2===argc) for the given adapter. Returns func on success or
     throws if (A) called with 2 args but adapter is-not-a Function or
     (B) typeName is not a string or (C) argc is not one of (1, 2).
  */
  const __xAdapter = function(func, argc, typeName, adapter, modeName, xcvPart){
    if('string'===typeof typeName){
      if(1===argc) return xcvPart.get(typeName);
      else if(2===argc){
        if(!adapter){
          xcvPart.delete(typeName);
          return func;
        }else if(!(adapter instanceof Function)){
          toss(modeName,"requires a function argument.");
        }
        xcvPart.set(typeName, adapter);
        return func;
      }
    }
    toss("Invalid arguments to",modeName);
  };

  /**
     Gets, sets, or removes a result value adapter for use with
     xWrap(). If passed only 1 argument, the adapter function for the
     given type name is returned.  If the second argument is explicit
     falsy (as opposed to defaulted), the adapter named by the first
     argument is removed. If the 2nd argument is not falsy, it must be
     a function which takes one value and returns a value appropriate
     for the given type name. The adapter may throw if its argument is
     not of a type it can work with. This function throws for invalid
     arguments.

     Example:

     ```
     xWrap.resultAdapter('twice',(v)=>v+v);
     ```

     Result adapters MUST NOT use the scopedAlloc() family of APIs to
     allocate a result value. xWrap()-generated wrappers run in the
     context of scopedAllocPush() so that argument adapters can easily
     convert, e.g., to C-strings, and have them cleaned up
     automatically before the wrapper returns to the caller. Likewise,
     if a _result_ adapter uses scoped allocation, the result will be
     freed before the wrapper returns, leading to chaos and undefined
     behavior.

     When called as a setter, this function returns itself.
  */
  target.xWrap.resultAdapter = function f(typeName, adapter){
    return __xAdapter(f, arguments.length, typeName, adapter,
                      'resultAdapter()', xResult);
  };

  /**
     Functions identically to xWrap.resultAdapter() but applies to
     call argument conversions instead of result value conversions.

     xWrap()-generated wrappers perform argument conversion in the
     context of a scopedAllocPush(), so any memory allocation
     performed by argument adapters really, really, really should be
     made using the scopedAlloc() family of functions unless
     specifically necessary. For example:

     ```
     xWrap.argAdapter('my-string', function(v){
       return ('string'===typeof v)
         ? myWasmObj.scopedAllocCString(v) : null;
     };
     ```

     Contrariwise, _result_ adapters _must not_ use scopedAlloc() to
     allocate results because they would be freed before the
     xWrap()-created wrapper returns.

     It is perfectly legitimate to use these adapters to perform
     argument validation, as opposed (or in addition) to conversion.
     When used that way, they should throw for invalid arguments.
  */
  target.xWrap.argAdapter = function f(typeName, adapter){
    return __xAdapter(f, arguments.length, typeName, adapter,
                      'argAdapter()', xArg);
  };

  target.xWrap.FuncPtrAdapter = xArg.FuncPtrAdapter;

  /**
     Functions like xCall() but performs argument and result type
     conversions as for xWrap(). The first, second, and third
     arguments are as documented for xWrap(), except that the 3rd
     argument may be either a falsy value or empty array to represent
     nullary functions. The 4th+ arguments are arguments for the call,
     with the special case that if the 4th argument is an array, it is
     used as the arguments for the call. Returns the converted result
     of the call.

     This is just a thin wrapper around xWrap(). If the given function
     is to be called more than once, it's more efficient to use
     xWrap() to create a wrapper, then to call that wrapper as many
     times as needed. For one-shot calls, however, this variant is
     simpler.
  */
  target.xCallWrapped = function(fArg, resultType, argTypes, ...args){
    if(Array.isArray(arguments[3])) args = arguments[3];
    return target.xWrap(fArg, resultType, argTypes||[]).apply(null, args||[]);
  };

  /**
     This function is ONLY exposed in the public API to facilitate
     testing. It should not be used in application-level code, only
     in test code.

     Expects to be given (typeName, value) and returns a conversion
     of that value as has been registered using argAdapter().
     It throws if no adapter is found.

     ACHTUNG: the adapter may require that a scopedAllocPush() is
     active and it may allocate memory within that scope. It may also
     require additional arguments, depending on the type of
     conversion.
  */
  target.xWrap.testConvertArg = cache.xWrap.convertArg;

  /**
     This function is ONLY exposed in the public API to facilitate
     testing. It should not be used in application-level code, only
     in test code.

     Expects to be given (typeName, value) and returns a conversion
     of that value as has been registered using resultAdapter().
     It throws if no adapter is found.

     ACHTUNG: the adapter may allocate memory which the caller may need
     to know how to free.
  */
  target.xWrap.testConvertResult = cache.xWrap.convertResult;

  return target;
};

/**
   yawl (Yet Another Wasm Loader) provides very basic wasm loader.
   It requires a config object:

   - `uri`: required URI of the WASM file to load.

   - `onload(loadResult)`: optional callback. Its argument is an
     object described in more detail below.

   - `imports`: optional imports object for
     WebAssembly.instantiate[Streaming]().  The default is an empty
     set of imports. If the module requires any imports, this object
     must include them.

   - `wasmUtilTarget`: optional object suitable for passing to
     WhWasmUtilInstaller(). If set, it gets passed to that function
     before the returned promise resolves. This function sets several
     properties on it before passing it on to that function (which
     sets many more):

     - `module`, `instance`: the properties from the
       instantiate[Streaming]() result.

     - If `instance.exports.memory` is _not_ set then it requires that
       `config.imports.env.memory` be set (else it throws), and
       assigns that to `wasmUtilTarget.memory`.

     - If `wasmUtilTarget.alloc` is not set and
       `instance.exports.malloc` is, it installs
       `wasmUtilTarget.alloc()` and `wasmUtilTarget.dealloc()`
       wrappers for the exports' `malloc` and `free` functions
       if exports.malloc exists.

   It returns a function which, when called, initiates loading of the
   module and returns a Promise. When that Promise resolves, it calls
   the `config.onload` callback (if set) and passes it `(loadResult)`,
   where `loadResult` is derived from the result of
   WebAssembly.instantiate[Streaming](), an object in the form:

   ```
   {
     module: a WebAssembly.Module,
     instance: a WebAssembly.Instance,
     config: the config arg to this function
   }
   ```

   (The initial `then()` attached to the promise gets only that
   object, and not the `config` object, thus the potential need for a
   `config.onload` handler.)

   Error handling is up to the caller, who may attach a `catch()` call
   to the promise.
*/
globalThis.WhWasmUtilInstaller
.yawl = function yawl(config){
  'use strict';
  const wfetch = ()=>fetch(config.uri, {credentials: 'same-origin'});
  const wui = this;
  const finalThen = function(arg){
    //log("finalThen()",arg);
    if(config.wasmUtilTarget){
      const toss = (...args)=>{throw new Error(args.join(' '))};
      const tgt = config.wasmUtilTarget;
      tgt.module = arg.module;
      tgt.instance = arg.instance;
      //tgt.exports = tgt.instance.exports;
      if(!tgt.instance.exports.memory){
        /**
           WhWasmUtilInstaller requires either tgt.exports.memory
           (exported from WASM) or tgt.memory (JS-provided memory
           imported into WASM).
        */
        tgt.memory = config?.imports?.env?.memory
          || toss("Missing 'memory' object!");
      }
      if(!tgt.alloc && arg.instance.exports.malloc){
        const exports = arg.instance.exports;
        tgt.alloc = function(n){
          return exports.malloc(n) || toss("Allocation of",n,"bytes failed.");
        };
        tgt.dealloc = function(m){m && exports.free(m)};
      }
      wui(tgt);
    }
    arg.config = config;
    if(config.onload) config.onload(arg);
    return arg /* for any then() handler attached to
                  yetAnotherWasmLoader()'s return value */;
  };
  const loadWasm = WebAssembly.instantiateStreaming
        ? ()=>WebAssembly
        .instantiateStreaming(wfetch(), config.imports||{})
        .then(finalThen)
        : ()=> wfetch()// Safari < v15
        .then(response => response.arrayBuffer())
        .then(bytes => WebAssembly.instantiate(bytes, config.imports||{}))
        .then(finalThen)
  ;
  return loadWasm;
}.bind(
globalThis.WhWasmUtilInstaller
)/*yawl()*/;
/**
  2022-06-30

  The author disclaims copyright to this source code.  In place of a
  legal notice, here is a blessing:

  *   May you do good and not evil.
  *   May you find forgiveness for yourself and forgive others.
  *   May you share freely, never taking more than you give.

  ***********************************************************************

  The Jaccwabyt API is documented in detail in an external file,
  _possibly_ called jaccwabyt.md in the same directory as this file.

  Project homes:
  - https://fossil.wanderinghorse.net/r/jaccwabyt
  - https://sqlite.org/src/dir/ext/wasm/jaccwabyt

  This build was generated using:

  ./c-pp -o js/jaccwabyt.js -@policy=error jaccwabyt/jaccwabyt.c-pp.js

  by libcmpp 2.x 2fc4afc31f6505c27b9c34988973a2bd9b157d559247cdd26868ae75632c3a5e @ 2025-11-16 23:03:27.352 UTC
*/
'use strict';
globalThis.Jaccwabyt =
function StructBinderFactory(config){
  'use strict';
/* ^^^^ it is recommended that clients move that object into wherever
   they'd like to have it and delete the globalThis-held copy.  This
   API does not require the global reference - it is simply installed
   as a convenience for connecting these bits to other co-developed
   code before it gets removed from the global namespace.
*/

  /** Throws a new Error, the message of which is the concatenation
      all args with a space between each. */
  const toss = (...args)=>{throw new Error(args.join(' '))};

  {
    let h = config.heap;
    if( h instanceof WebAssembly.Memory ){
      h = function(){return new Uint8Array(this.buffer)}.bind(h);
    }else if( !(h instanceof Function) ){
      //console.warn("The bothersome StructBinderFactory config:",config);
      toss("config.heap must be WebAssembly.Memory instance or",
           "a function which returns one.");
    }
    config.heap = h;
  }
  ['alloc','dealloc'].forEach(function(k){
    (config[k] instanceof Function) ||
      toss("Config option '"+k+"' must be a function.");
  });
  const SBF = StructBinderFactory;
  const heap = config.heap,
        alloc = config.alloc,
        dealloc = config.dealloc,
        realloc = (config.realloc || function(){
          toss("This StructBinderFactory was configured without realloc()");
          /* We can't know the original memory's size from here unless
             we internally proxy alloc()/dealloc() to track all
             pointers (not going to happen), so we can't fall back to
             doing alloc()/copy/dealloc(). */
        }),
        log = config.log || console.debug.bind(console),
        memberPrefix = (config.memberPrefix || ""),
        memberSuffix = (config.memberSuffix || ""),
        BigInt = globalThis['BigInt'],
        BigInt64Array = globalThis['BigInt64Array'],
        bigIntEnabled = config.bigIntEnabled ?? !!BigInt64Array;

  //console.warn("config",config);
  let ptr;
  const ptrSize = config.pointerSize
        || config.ptrSize/*deprecated*/
        || ('bigint'===typeof (ptr = alloc(1)) ? 8 : 4);
  const ptrIR = config.pointerIR/*deprecated*/
        || config.ptrIR/*deprecated*/
        || (4===ptrSize ? 'i32' : 'i64');
  if( ptr ){
    dealloc(ptr);
    ptr = undefined;
  }
  //console.warn("ptrIR =",ptrIR,"ptrSize =",ptrSize);

  if(ptrSize!==4 && ptrSize!==8) toss("Invalid pointer size:",ptrSize);
  if(ptrIR!=='i32' && ptrIR!=='i64') toss("Invalid pointer representation:",ptrIR);

  /** Either BigInt or, if !bigIntEnabled, a function which
      throws complaining that BigInt is not enabled. */
  const __BigInt = (bigIntEnabled && BigInt)
        ? (v)=>BigInt(v || 0)
        : (v)=>toss("BigInt support is disabled in this build.");
  const __asPtrType = ('i32'==ptrIR) ? Number : __BigInt;
  const __NullPtr = __asPtrType(0);

  /**
     Expects any number of numeric arguments, each one of either type
     Number or BigInt. It sums them up (from an implicit starting
     point of 0 or 0n) and returns them as a number of the same type
     which target.asPtrType() uses.

     This is a workaround for not being able to mix Number/BigInt in
     addition/subtraction expressions (which we frequently need for
     calculating pointer offsets).
  */
  const __ptrAdd = function(...args){
    let rc = __NullPtr;
    for( let i = 0; i < args.length; ++i ){
      rc += __asPtrType(args[i]);
    }
    return rc;
  };

  const __ptrAddSelf = function(...args){
    return __ptrAdd(this.pointer,...args);
  };

  if(!SBF.debugFlags){
    SBF.__makeDebugFlags = function(deriveFrom=null){
      /* This is disgustingly overengineered. :/ */
      if(deriveFrom && deriveFrom.__flags) deriveFrom = deriveFrom.__flags;
      const f = function f(flags){
        if(0===arguments.length){
          return f.__flags;
        }
        if(flags<0){
          delete f.__flags.getter; delete f.__flags.setter;
          delete f.__flags.alloc; delete f.__flags.dealloc;
        }else{
          f.__flags.getter  = 0!==(0x01 & flags);
          f.__flags.setter  = 0!==(0x02 & flags);
          f.__flags.alloc   = 0!==(0x04 & flags);
          f.__flags.dealloc = 0!==(0x08 & flags);
        }
        return f._flags;
      };
      Object.defineProperty(f,'__flags', {
        iterable: false, writable: false,
        value: Object.create(deriveFrom)
      });
      if(!deriveFrom) f(0);
      return f;
    };
    SBF.debugFlags = SBF.__makeDebugFlags();
  }/*static init*/

  const isLittleEndian = true || (function() {
    const buffer = new ArrayBuffer(2);
    new DataView(buffer).setInt16(0, 256, true /* littleEndian */);
    // Int16Array uses the platform's endianness.
    return new Int16Array(buffer)[0] === 256;
  })() /* WASM is, by definition, Little Endian */;

  /**
     Some terms used in the internal docs:

     StructType: a struct-wrapping class generated by this
     framework.

     DEF: struct description object.

     SIG: struct member signature string.
  */

  /** True if SIG s looks like a function signature, else
      false. */
  const isFuncSig = (s)=>'('===s[1];
  /** True if SIG s is-a pointer-type signature. */
  const isPtrSig = (s)=>'p'===s || 'P'===s || 's'===s;
  const isAutoPtrSig = (s)=>'P'===s /*EXPERIMENTAL*/;
  /** Returns p if SIG s is a function SIG, else returns s[0]. */
  const sigLetter = (s)=>s ? (isFuncSig(s) ? 'p' : s[0]) : undefined;

  /** Returns the WASM IR form of the letter at SIG s[0]. Throws for
      an unknown SIG. */
  const sigIR = function(s){
    switch(sigLetter(s)){
        case 'c': case 'C': return 'i8';
        case 'i': return 'i32';
        case 'p': case 'P': case 's': return ptrIR;
        case 'j': return 'i64';
        case 'f': return 'float';
        case 'd': return 'double';
    }
    toss("Unhandled signature IR:",s);
  };

  /** Returns the WASM sizeof of the letter at SIG s[0]. Throws for an
      unknown SIG. */
  const sigSize = function(s){
    switch(sigLetter(s)){
        case 'c': case 'C': return 1;
        case 'i': return 4;
        case 'p': case 'P': case 's': return ptrSize;
        case 'j': return 8;
        case 'f': return 4;
        case 'd': return 8;
    }
    toss("Unhandled signature sizeof:",s);
  };

  const affirmBigIntArray = BigInt64Array
        ? ()=>true : ()=>toss('BigInt64Array is not available.');

  /** Returns the name of a DataView getter method corresponding
      to the given SIG. */
  const sigDVGetter = function(s){
    switch(sigLetter(s)) {
        case 'p': case 'P': case 's': {
          switch(ptrSize){
              case 4: return 'getInt32';
              case 8: return affirmBigIntArray() && 'getBigInt64';
          }
          break;
        }
        case 'i': return 'getInt32';
        case 'c': return 'getInt8';
        case 'C': return 'getUint8';
        case 'j': return affirmBigIntArray() && 'getBigInt64';
        case 'f': return 'getFloat32';
        case 'd': return 'getFloat64';
    }
    toss("Unhandled DataView getter for signature:",s);
  };

  /** Returns the name of a DataView setter method corresponding
      to the given SIG. */
  const sigDVSetter = function(s){
    switch(sigLetter(s)){
        case 'p': case 'P': case 's': {
          switch(ptrSize){
              case 4: return 'setInt32';
              case 8: return affirmBigIntArray() && 'setBigInt64';
          }
          break;
        }
        case 'i': return 'setInt32';
        case 'c': return 'setInt8';
        case 'C': return 'setUint8';
        case 'j': return affirmBigIntArray() && 'setBigInt64';
        case 'f': return 'setFloat32';
        case 'd': return 'setFloat64';
    }
    toss("Unhandled DataView setter for signature:",s);
  };

  /**
     Returns a factory for either Number or BigInt, depending on the
     given SIG. This constructor is used in property setters to coerce
     the being-set value to the correct pointer size.
  */
  const sigDVSetWrapper = function(s){
    switch(sigLetter(s)) {
        case 'i': case 'f': case 'c': case 'C': case 'd': return Number;
        case 'j': return __BigInt;
        case 'p': case 'P': case 's':
          switch(ptrSize){
              case 4: return Number;
              case 8: return __BigInt;
          }
          break;
    }
    toss("Unhandled DataView set wrapper for signature:",s);
  };

  /** Returns the given struct and member name in a form suitable for
      debugging and error output. */
  const sPropName = (s,k)=>s+'::'+k;

  const __propThrowOnSet = function(structName,propName){
    return ()=>toss(sPropName(structName,propName),"is read-only.");
  };

  /**
     In order to completely hide StructBinder-bound struct pointers
     from JS code, we store them in a scope-local WeakMap which maps
     the struct-bound objects to an object with their metadata:

     {
       .p = the native pointer,
       .o = self (for an eventual reverse-mapping),
       .xb = extra bytes allocated for p,
       .zod = zeroOnDispose,
       .ownsPointer = true if this object owns p
     }

     The .p data are accessible via obj.pointer, which is gated behind
     a property interceptor, but are not exposed anywhere else in the
     public API.
  */
  const getInstanceHandle = function f(obj, create=true){
    let ii = f.map.get(obj);
    if( !ii && create ){
      f.map.set(obj, (ii=f.create(obj)));
    }
    return ii;
  };
  getInstanceHandle.map = new WeakMap;
  getInstanceHandle.create = (forObj)=>{
    return Object.assign(Object.create(null),{
      o: forObj,
      p: undefined/*native ptr*/,
      ownsPointer: false,
      zod: false/*zeroOnDispose*/,
      xb: 0/*extraBytes*/
    });
  };

  /**
     Remove the getInstanceHandle() mapping for obj.
  */
  const rmInstanceHandle = (obj)=>getInstanceHandle.map.delete(obj)
    /* If/when we have a reverse map of ptr-to-objects, we need to
       clean that here. */;

  const __isPtr32 = (ptr)=>('number'===typeof ptr && (ptr===(ptr|0)) && ptr>=0);
  const __isPtr64 = (ptr)=>(
    ('bigint'===typeof ptr && ptr >= 0) || __isPtr32(ptr)
  );

  /**
     isPtr() is an alias for isPtr32() or isPtr64(), depending on
     ptrSize.
  */
  const __isPtr = (4===ptrSize) ? __isPtr32 : __isPtr64;

  const __isNonNullPtr = (v)=>__isPtr(v) && (v>0);

  /** Frees the obj.pointer memory (a.k.a. m), handles obj.ondispose,
      and unmaps obj from its native resources. */
  const __freeStruct = function(ctor, obj, m){
    const ii = getInstanceHandle(obj, false);
    if( !ii ) return;
    rmInstanceHandle(obj);
    if( !m && !(m = ii.p) ){
      console.warn("Cannot(?) happen: __freeStruct() found no instanceInfo");
      return;
    }
    if(Array.isArray(obj.ondispose)){
      let x;
      while((x = obj.ondispose.pop())){
        try{
          if(x instanceof Function) x.call(obj);
          else if(x instanceof StructType) x.dispose();
          else if(__isPtr(x)) dealloc(x);
          // else ignore. Strings are permitted to annotate entries
          // to assist in debugging.
        }catch(e){
          console.warn("ondispose() for",ctor.structName,'@',
                       m,'threw. NOT propagating it.',e);
        }
      }
    }else if(obj.ondispose instanceof Function){
      try{obj.ondispose()}
      catch(e){
        /*do not rethrow: destructors must not throw*/
        console.warn("ondispose() for",ctor.structName,'@',
                     m,'threw. NOT propagating it.',e);
      }
    }
    delete obj.ondispose;
    if(ctor.debugFlags.__flags.dealloc){
      log("debug.dealloc:",(ii.ownsPointer?"":"EXTERNAL"),
          ctor.structName,"instance:",
          ctor.structInfo.sizeof,"bytes @"+m);
    }
    if(ii.ownsPointer){
      if( ii.zod || ctor.structInfo.zeroOnDispose ){
        heap().fill(0, Number(m),
                    Number(m) + ctor.structInfo.sizeof + ii.xb);
      }
      dealloc(m);
    }
  };

  /** Returns a skeleton for a read-only, non-iterable property
   * descriptor. */
  const rop0 = ()=>{return {configurable: false, writable: false,
                            iterable: false}};

  /** Returns a skeleton for a read-only property accessor wrapping
      value v. */
  const rop = (v)=>{return {...rop0(), value: v}};

  /** Allocates obj's memory buffer based on the size defined in
      ctor.structInfo.sizeof. */
  const __allocStruct = function f(ctor, obj, xm){
    let opt;
    const checkPtr = (ptr)=>{
      __isNonNullPtr(ptr) ||
        toss("Invalid pointer value",arguments[0],"for",ctor.structName,"constructor.");
    };
    if( arguments.length>=3 ){
      if( xm && ('object'===typeof xm) ){
        opt = xm;
        xm = opt?.wrap;
      }else{
        checkPtr(xm);
        opt = {wrap: xm};
      }
    }else{
      opt = {}
    }

    const fill = !xm /* true if we need to zero the memory */;
    let nAlloc = 0;
    let ownsPointer = false;
    if(xm){
      /* Externally-allocated memory. */
      checkPtr(xm);
      ownsPointer = !!opt?.takeOwnership;
    }else{
      const nX = opt?.extraBytes ?? 0;
      if( nX<0 || (nX!==(nX|0)) ){
        toss("Invalid extraBytes value:",opt?.extraBytes);
      }
      nAlloc = ctor.structInfo.sizeof + nX;
      xm = alloc(nAlloc)
        || toss("Allocation of",ctor.structName,"structure failed.");
      ownsPointer = true;
    }
    try {
      if( opt?.debugFlags ){
        /* specifically undocumented */
        obj.debugFlags(opt.debugFlags);
      }
      if(ctor./*prototype.???*/debugFlags.__flags.alloc){
        log("debug.alloc:",(fill?"":"EXTERNAL"),
            ctor.structName,"instance:",
            ctor.structInfo.sizeof,"bytes @"+xm);
      }
      if(fill){
        heap().fill(0, Number(xm), Number(xm) + nAlloc);
      }
      const ii = getInstanceHandle(obj);
      ii.p = xm;
      ii.ownsPointer = ownsPointer;
      ii.xb = nAlloc ? (nAlloc-ctor.structInfo.sizeof) : 0;
      ii.zod = !!opt?.zeroOnDispose;
      if( opt?.ondispose && opt.ondispose!==xm ){
        obj.addOnDispose( opt.ondispose );
      }
    }catch(e){
      __freeStruct(ctor, obj, xm);
      throw e;
    }
  };

  /** True if sig looks like an emscripten/jaccwabyt
      type signature, else false. */
  const looksLikeASig = function f(sig){
    f.rxSig1 ??= /^[ipPsjfdcC]$/;
    f.rxSig2 ??= /^[vipPsjfdcC]\([ipPsjfdcC]*\)$/;
    return f.rxSig1.test(sig) || f.rxSig2.test(sig);
  };

  /** Returns a pair of adaptor maps (objects) in a length-3
      array specific to the given object. */
  const __adaptorsFor = function(who){
    let x = this.get(who);
    if( !x ){
      x = [ Object.create(null), Object.create(null), Object.create(null) ];
      this.set(who, x);
    }
    return x;
  }.bind(new WeakMap);

  /** Code de-duplifier for __adaptGet(), __adaptSet(), and
      __adaptStruct(). */
  const __adaptor = function(who, which, key, proxy){
    const a = __adaptorsFor(who)[which];
    if(3===arguments.length) return a[key];
    if( proxy ) return a[key] = proxy;
    return delete a[key];
  };

  const noopAdapter = (x)=>x;

  // StructBinder::adaptGet()
  const __adaptGet = function(key, ...args){
    return __adaptor(this, 0, key, ...args);
  };

  const __affirmNotASig = function(ctx,key){
    looksLikeASig(key) &&
      toss(ctx,"(",key,") collides with a data type signature.");
  };

  // StructBinder::adaptSet()
  const __adaptSet = function(key, ...args){
    __affirmNotASig('Setter adaptor',key);
    return __adaptor(this, 1, key, ...args);
  };

  // StructBinder::adaptStruct()
  const __adaptStruct = function(key, ...args){
    __affirmNotASig('Struct adaptor',key);
    return __adaptor(this, 2, key, ...args);
  };

  /**
     An internal counterpart of __adaptStruct().  If key is-a string,
     uses __adaptor(who) to fetch the struct-adaptor entry for key,
     else key is assumed to be a struct description object. If it
     resolves to an object, that's returned, else an exception is
     thrown.
  */
  const __adaptStruct2 = function(who,key){
    const si = ('string'===typeof key)
          ? __adaptor(who, 2, key) : key;
    if( 'object'!==typeof si ){
      toss("Invalid struct mapping object. Arg =",key,JSON.stringify(si));
    }
    return si;
  };

  const __memberKey = (k)=>memberPrefix + k + memberSuffix;
  const __memberKeyProp = rop(__memberKey);
  //const __adaptGetProp = rop(__adaptGet);

  /**
     Looks up a struct member in structInfo.members. Throws if found
     if tossIfNotFound is true, else returns undefined if not
     found. The given name may be either the name of the
     structInfo.members key (faster) or the key as modified by the
     memberPrefix and memberSuffix settings.
  */
  const __lookupMember = function(structInfo, memberName, tossIfNotFound=true){
    let m = structInfo.members[memberName];
    if(!m && (memberPrefix || memberSuffix)){
      // Check for a match on members[X].key
      for(const v of Object.values(structInfo.members)){
        if(v.key===memberName){ m = v; break; }
      }
      if(!m && tossIfNotFound){
        toss(sPropName(structInfo.name || structInfo.structName, memberName),
             'is not a mapped struct member.');
      }
    }
    return m;
  };

  /**
     Uses __lookupMember(obj.structInfo,memberName) to find a member,
     throwing if not found. Returns its signature, either in this
     framework's native format or in Emscripten format.
  */
  const __memberSignature = function f(obj,memberName,emscriptenFormat=false){
    if(!f._) f._ = (x)=>x.replace(/[^vipPsjrdcC]/g,"").replace(/[pPscC]/g,'i');
    const m = __lookupMember(obj.structInfo, memberName, true);
    return emscriptenFormat ? f._(m.signature) : m.signature;
  };

  /** Impl of X.memberKeys() for StructType and struct ctors. */
  const __structMemberKeys = rop(function(){
    const a = [];
    for(const k of Object.keys(this.structInfo.members)){
      a.push(this.memberKey(k));
    }
    return a;
  });

  const __utf8Decoder = new TextDecoder('utf-8');
  const __utf8Encoder = new TextEncoder();
  /** Internal helper to use in operations which need to distinguish
      between SharedArrayBuffer heap memory and non-shared heap. */
  const __SAB = ('undefined'===typeof SharedArrayBuffer)
        ? function(){} : SharedArrayBuffer;
  const __utf8Decode = function(arrayBuffer, begin, end){
    if( 8===ptrSize ){
      begin = Number(begin);
      end = Number(end);
    }
    return __utf8Decoder.decode(
      (arrayBuffer.buffer instanceof __SAB)
        ? arrayBuffer.slice(begin, end)
        : arrayBuffer.subarray(begin, end)
    );
  };

  /**
     Uses __lookupMember() to find the given obj.structInfo key.
     Returns that member if it is a string, else returns false. If the
     member is not found, throws if tossIfNotFound is true, else
     returns false.
   */
  const __memberIsString = function(obj,memberName, tossIfNotFound=false){
    const m = __lookupMember(obj.structInfo, memberName, tossIfNotFound);
    return (m && 1===m.signature.length && 's'===m.signature[0]) ? m : false;
  };

  /**
     Given a member description object, throws if member.signature is
     not valid for assigning to or interpretation as a C-style string.
     It optimistically assumes that any signature of (i,p,s) is
     C-string compatible.
  */
  const __affirmCStringSignature = function(member){
    if('s'===member.signature) return;
    toss("Invalid member type signature for C-string value:",
         JSON.stringify(member));
  };

  /**
     Looks up the given member in obj.structInfo. If it has a
     signature of 's' then it is assumed to be a C-style UTF-8 string
     and a decoded copy of the string at its address is returned. If
     the signature is of any other type, it throws. If an s-type
     member's address is 0, `null` is returned.
  */
  const __memberToJsString = function f(obj,memberName){
    const m = __lookupMember(obj.structInfo, memberName, true);
    __affirmCStringSignature(m);
    const addr = obj[m.key];
    //log("addr =",addr,memberName,"m =",m);
    if(!addr) return null;
    let pos = addr;
    const mem = heap();
    for( ; mem[pos]!==0; ++pos ) {
      //log("mem[",pos,"]",mem[pos]);
    };
    //log("addr =",addr,"pos =",pos);
    return (addr===pos) ? "" : __utf8Decode(mem, addr, pos);
  };

  /**
     Adds value v to obj.ondispose, creating ondispose,
     or converting it to an array, if needed.
  */
  const __addOnDispose = function(obj, ...v){
    if(obj.ondispose){
      if(!Array.isArray(obj.ondispose)){
        obj.ondispose = [obj.ondispose];
      }
    }else{
      obj.ondispose = [];
    }
    obj.ondispose.push(...v);
  };

  /**
     Allocates a new UTF-8-encoded, NUL-terminated copy of the given
     JS string and returns its address relative to heap(). If
     allocation returns 0 this function throws. Ownership of the
     memory is transfered to the caller, who must eventually pass it
     to the configured dealloc() function.
  */
  const __allocCString = function(str){
    const u = __utf8Encoder.encode(str);
    const mem = alloc(u.length+1);
    if(!mem) toss("Allocation error while duplicating string:",str);
    const h = heap();
    //let i = 0;
    //for( ; i < u.length; ++i ) h[mem + i] = u[i];
    h.set(u, Number(mem));
    h[__ptrAdd(mem, u.length)] = 0;
    //log("allocCString @",mem," =",u);
    return mem;
  };

  /**
     Sets the given struct member of obj to a dynamically-allocated,
     UTF-8-encoded, NUL-terminated copy of str. It is up to the caller
     to free any prior memory, if appropriate. The newly-allocated
     string is added to obj.ondispose so will be freed when the object
     is disposed.

     The given name may be either the name of the structInfo.members
     key (faster) or the key as modified by the memberPrefix and
     memberSuffix settings.
  */
  const __setMemberCString = function(obj, memberName, str){
    const m = __lookupMember(obj.structInfo, memberName, true);
    __affirmCStringSignature(m);
    /* Potential TODO: if obj.ondispose contains obj[m.key] then
       dealloc that value and clear that ondispose entry */
    const mem = __allocCString(str);
    obj[m.key] = mem;
    __addOnDispose(obj, mem);
    return obj;
  };

  /**
     Prototype for all StructFactory instances (the constructors
     returned from StructBinder).
  */
  const StructType = function StructType(structName, structInfo){
    if(arguments[2]!==rop/*internal sentinel value*/){
      toss("Do not call the StructType constructor",
           "from client-level code.");
    }
    Object.defineProperties(this,{
      //isA: rop((v)=>v instanceof StructType),
      structName: rop(structName),
      structInfo: rop(structInfo)
    });
  };

  /**
     Properties inherited by struct-type-specific StructType instances
     and (indirectly) concrete struct-type instances.
  */
  StructType.prototype = Object.create(null, {
    dispose: rop(function(){__freeStruct(this.constructor, this)}),
    lookupMember: rop(function(memberName, tossIfNotFound=true){
      return __lookupMember(this.structInfo, memberName, tossIfNotFound);
    }),
    memberToJsString: rop(function(memberName){
      return __memberToJsString(this, memberName);
    }),
    memberIsString: rop(function(memberName, tossIfNotFound=true){
      return __memberIsString(this, memberName, tossIfNotFound);
    }),
    memberKey: __memberKeyProp,
    memberKeys: __structMemberKeys,
    memberSignature: rop(function(memberName, emscriptenFormat=false){
      return __memberSignature(this, memberName, emscriptenFormat);
    }),
    memoryDump: rop(function(){
      const p = this.pointer;
      return p
        ? new Uint8Array(heap().slice(Number(p), Number(p) + this.structInfo.sizeof))
        : null;
    }),
    extraBytes: {
      configurable: false, enumerable: false,
      get: function(){return getInstanceHandle(this, false)?.xb ?? 0;}
    },
    zeroOnDispose: {
      configurable: false, enumerable: false,
      get: function(){
        return getInstanceHandle(this, false)?.zod
          ?? !!this.structInfo.zeroOnDispose;
      }
    },
    pointer: {
      configurable: false, enumerable: false,
      get: function(){return getInstanceHandle(this, false)?.p},
      set: ()=>toss("Cannot assign the 'pointer' property of a struct.")
      // Reminder: leaving `set` undefined makes assignments
      // to the property _silently_ do nothing. Current unit tests
      // rely on it throwing, though.
    },
    setMemberCString: rop(function(memberName, str){
      return __setMemberCString(this, memberName, str);
    })
  });
  // Function-type non-Property inherited members
  Object.assign(StructType.prototype,{
    addOnDispose: function(...v){
      __addOnDispose(this,...v);
      return this;
    }
  });

  /**
     "Static" properties for StructType.
  */
  Object.defineProperties(StructType, {
    allocCString: rop(__allocCString),
    isA: rop((v)=>v instanceof StructType),
    hasExternalPointer: rop((v)=>{
      const ii = getInstanceHandle(v, false);
      return !!(ii?.p && !ii?.ownsPointer);
    }),
    memberKey: __memberKeyProp
    //ptrAdd = rop(__ptrAdd) no b/c one might think that it adds based on this.pointer.
  });

  /**
     If struct description object si has a getter proxy, return it (a
     function), else return undefined.
  */
  const memberGetterProxy = function(si){
    return si.get || (si.adaptGet
                      ? StructBinder.adaptGet(si.adaptGet)
                      : undefined);
  };

  /**
     If struct description object si has a setter proxy, return it (a
     function), else return undefined.
  */
  const memberSetterProxy = function(si){
    return si.set || (si.adaptSet
                      ? StructBinder.adaptSet(si.adaptSet)
                      : undefined);
  };

  /**
     To be called by makeMemberWrapper() when si has a 'members'
     member, i.e. is an embedded struct. This function sets up that
     struct like any other and also sets up property accessor for
     ctor.memberKey(name) which returns an instance of that new
     StructType when the member is accessed. That instance wraps the
     memory of the member's part of the containing C struct instance.

     That is, if struct Foo has member bar which is an inner struct
     then:

     const f = new Foo;
     const b = f.bar;
     assert( b is-a StructType object );
     assert( b.pointer === f.b.pointer );

     b will be disposed of when f() is. Calling b.dispose() will not
     do any permanent harm, as the wrapper object will be recreated
     when accessing f.bar, pointing to the same memory in f.

     The si.zeroOnDispose flag has no effect on embedded structs because
     they wrap "external" memory, so do not own it, and are thus never
     freed, as such.
  */
  const makeMemberStructWrapper = function callee(ctor, name, si){
    /**
       Where we store inner-struct member proxies. Keys are a
       combination of the parent object's pointer address and the
       property's name. The values are StructType instances.
    */
    const __innerStructs = (callee.innerStructs ??= new Map());
    const key = ctor.memberKey(name);
    if( undefined!==si.signature ){
      toss("'signature' cannot be used on an embedded struct (",
           ctor.structName,".",key,").");
    }
    if( memberSetterProxy(si) ){
      toss("'set' and 'adaptSet' are not permitted for nested struct members.");
    }
    //console.warn("si =",ctor.structName, name, JSON.stringify(si,'  '));
    si.structName ??= ctor.structName+'::'+name;
    si.key = key;
    si.name = name;
    si.constructor = this.call(this, si.structName, si);
    //console.warn("si.constructor =",si.constructor);
    //console.warn("si =",si,"ctor=",ctor);
    const getterProxy = memberGetterProxy(si);
    const prop = Object.assign(Object.create(null),{
      configurable: false,
      enumerable: false,
      set: __propThrowOnSet(ctor/*not si.constructor*/.structName, key),
      get: function(){
        const dbg = this.debugFlags.__flags;
        const p = this.pointer;
        const k = p+'.'+key;
        let s = __innerStructs.get(k);
        if(dbg.getter){ log("debug.getter: k =",k); }
        if( !s ){
          s = new si.constructor(__ptrAdd(p, si.offset));
          __innerStructs.set(k, s);
          this.addOnDispose(()=>s.dispose());
          s.addOnDispose(()=>__innerStructs.delete(k));
          //console.warn("Created",k,"proxy");
        }
        if(getterProxy) s = getterProxy.apply(this,[s,key]);
        if(dbg.getter) log("debug.getter: result =",s);
        return s;
      }
    });
    Object.defineProperty(ctor.prototype, key, prop);
  }/*makeMemberStructWrapper()*/;

  /**
     This is where most of the magic happens.

     Pass this a StructBinderImpl-generated constructor, a member
     property name, and the struct member description object. It will
     define property accessors for proto[memberKey] which read
     from/write to memory in this.pointer. It modifies si to make
     certain downstream operations simpler.
  */
  const makeMemberWrapper = function f(ctor, name, si){
    si = __adaptStruct2(this, si);
    if( si.members ){
      return makeMemberStructWrapper.call(this, ctor, name, si);
    }

    if(!f.cache){
      /* Cache all available getters/setters/set-wrappers for
         direct reuse in each accessor function. */
      f.cache = {getters: {}, setters: {}, sw:{}};
      const a = ['i','c','C','p','P','s','f','d','v()'];
      if(bigIntEnabled) a.push('j');
      a.forEach(function(v){
        f.cache.getters[v] = sigDVGetter(v) /* DataView[MethodName] values for GETTERS */;
        f.cache.setters[v] = sigDVSetter(v) /* DataView[MethodName] values for SETTERS */;
        f.cache.sw[v] = sigDVSetWrapper(v)  /* BigInt or Number ctor to wrap around values
                                           for conversion */;
      });
      f.sigCheck = function(obj, name, key,sig){
        if(Object.prototype.hasOwnProperty.call(obj, key)){
          toss(obj.structName,'already has a property named',key+'.');
        }
        looksLikeASig(sig)
          || toss("Malformed signature for",
                  sPropName(obj.structName,name)+":",sig);
      };
    }
    const key = ctor.memberKey(name);
    f.sigCheck(ctor.prototype, name, key, si.signature);
    si.key = key;
    si.name = name;
    const sigGlyph = sigLetter(si.signature);
    const xPropName = sPropName(ctor.structName,key);
    const dbg = ctor.debugFlags.__flags;
    /*
      TODO?: set prototype of si to an object which can set/fetch
      its preferred representation, e.g. conversion to string or mapped
      function. Advantage: we can avoid doing that via if/else if/else
      in the get/set methods.
    */
    const getterProxy = memberGetterProxy(si);
    const prop = Object.create(null);
    prop.configurable = false;
    prop.enumerable = false;
    prop.get = function(){
      /**
         This getter proxy reads its value from the appropriate pointer
         address in the heap. It knows where and how much to read based on
         this.pointer, si.offset, and si.sizeof.
      */
      if(dbg.getter){
        log("debug.getter:",f.cache.getters[sigGlyph],"for", sigIR(sigGlyph),
            xPropName,'@', this.pointer,'+',si.offset,'sz',si.sizeof);
      }
      let rc = (
        new DataView(heap().buffer, Number(this.pointer) + si.offset, si.sizeof)
      )[f.cache.getters[sigGlyph]](0, isLittleEndian);

      if(getterProxy) rc = getterProxy.apply(this,[key,rc]);
      if(dbg.getter) log("debug.getter:",xPropName,"result =",rc);
      return rc;
    };
    if(si.readOnly){
      prop.set = __propThrowOnSet(ctor.prototype.structName,key);
    }else{
      const setterProxy = memberSetterProxy(si);
      prop.set = function(v){
        /**
           The converse of prop.get(), this encodes v into the appropriate
           spot in the WASM heap.
        */
        if(dbg.setter){
          log("debug.setter:",f.cache.setters[sigGlyph],"for", sigIR(sigGlyph),
              xPropName,'@', this.pointer,'+',si.offset,'sz',si.sizeof, v);
        }
        if(!this.pointer){
          toss("Cannot set native property on a disposed",
               this.structName,"instance.");
        }
        if( setterProxy ) v = setterProxy.apply(this,[key,v]);
        if( null===v || undefined===v ) v = __NullPtr;
        else if( isPtrSig(si.signature) && !__isPtr(v) ){
          if(isAutoPtrSig(si.signature) && (v instanceof StructType)){
            // It's a struct instance: store its pointer value
            v = v.pointer || __NullPtr;
            if(dbg.setter) log("debug.setter:",xPropName,"resolved to",v);
          }else{
            toss("Invalid value for pointer-type",xPropName+'.');
          }
        }
        (
          new DataView(heap().buffer, Number(this.pointer) + si.offset,
                       si.sizeof)
        )[f.cache.setters[sigGlyph]](0, f.cache.sw[sigGlyph](v), isLittleEndian);
      };
    }
    Object.defineProperty(ctor.prototype, key, prop);
  }/*makeMemberWrapper()*/;

  /**
     The main factory function which will be returned to the
     caller. The third argument is structly for internal use.

     This level of indirection is to avoid that clients can pass a
     third argument to this, as that's only for internal use.

     internalOpt options:

     - None right now. This is for potential use in recursion.

     Usages:

     StructBinder(string, obj [,optObj]);
     StructBinder(obj);
  */
  const StructBinderImpl = function StructBinderImpl(
    structName, si, opt = Object.create(null)
  ){
    /**
       StructCtor is the eventual return value of this function. We
       need to populate this early on so that we can do some trickery
       in feeding it through recursion.

       Uses:

       // heap-allocated:
       const x = new StructCtor;
       // externally-managed memory:
       const y = new StructCtor( aPtrToACompatibleCStruct );

       or, more recently:

       const z = new StructCtor({
         extraBytes: [int=0] extra bytes to allocate after the struct

         wrap: [aPtrToACompatibleCStruct=undefined]. If provided, this
         instance waps, but does not (by default) own the memory, else
         a new instance is allocated from the WASM heap.

         ownsPointer: true if this object takes over ownership of
         wrap.

         zeroOnDispose: [bool=StructCtor.structInfo.zeroOnDispose]

         autoCalcSizeOffset: [bool=false] Automatically calculate
         sizeof an offset. This is fine for pure-JS structs (which
         probably aren't useful beyond testing of Jaccwabyt) but it's
         dangerous to use with actual WASM objects because we cannot
         be guaranteed to have the same memory layout as an ostensibly
         matching C struct. This applies recursively to all children
         of the struct description.

         // TODO? Per-instance overrides of the struct-level flags?

         get: (k,v)=>v,
         set: (k,v)=>v,
         adaptGet: string,
         adaptSet: string

         // That wouldn't fit really well right now, apparently.
       });

    */
    const StructCtor = function StructCtor(arg){
      //console.warn("opt",opt,arguments[0]);
      if(!(this instanceof StructCtor)){
        toss("The",structName,"constructor may only be called via 'new'.");
      }
      __allocStruct(StructCtor, this, ...arguments);
    };
    const self = this;
    /**
      "Convert" struct description x to a struct description, if
      needed. This expands adaptStruct() mappings and transforms
      {memberName:signatureString} signature syntax to object form.
    */
    const ads = (x)=>{
      //console.warn("looksLikeASig(",x,") =",looksLikeASig(x));
      return (('string'===typeof x) && looksLikeASig(x))
        ? {signature: x} : __adaptStruct2(self,x);
    };
    if(1===arguments.length){
      si = ads(structName);
      structName = si.structName || si.name;
    }else if(2===arguments.length){
      si = ads(si);
      si.name ??= structName;
    }else{
      si = ads(si);
    }
    structName ??= si.structName;
    //console.warn("arguments =",JSON.stringify(arguments));
    structName ??= opt.structName;
    if( !structName ) toss("One of 'name' or 'structName' are required.");
    if( si.adapt ){
      /* Install adaptGet(), adaptSet(), and adaptStruct() proxies. */
      Object.keys(si.adapt.struct||{}).forEach((k)=>{
        __adaptStruct.call(StructBinderImpl, k, si.adapt.struct[k]);
      });
      Object.keys(si.adapt.set||{}).forEach((k)=>{
        __adaptSet.call(StructBinderImpl, k, si.adapt.set[k]);
      });
      Object.keys(si.adapt.get||{}).forEach((k)=>{
        __adaptGet.call(StructBinderImpl, k, si.adapt.get[k]);
      });
    }
    if(!si.members && !si.sizeof){
      si.sizeof = sigSize(si.signature);
    }

    const debugFlags = rop(SBF.__makeDebugFlags(StructBinder.debugFlags));
    Object.defineProperties(StructCtor,{
      debugFlags: debugFlags,
      isA: rop((v)=>v instanceof StructCtor),
      memberKey: __memberKeyProp,
      memberKeys: __structMemberKeys,
      //methodInfoForKey: rop(function(mKey){/*???*/}),
      structInfo: rop(si),
      structName: rop(structName),
      ptrAdd: rop(__ptrAdd)
    });
    StructCtor.prototype = new StructType(structName, si, rop);
    Object.defineProperties(StructCtor.prototype,{
      debugFlags: debugFlags,
      constructor: rop(StructCtor)
      /*if we assign StructCtor.prototype and don't do
        this then StructCtor!==instance.constructor*/,
      ptrAdd: rop(__ptrAddSelf)
    });
    let lastMember = false;
    let offset = 0;
    const autoCalc = !!si.autoCalcSizeOffset;
    //console.warn(structName,"si =",si);
    if( !autoCalc ){
      if( !si.sizeof ){
        toss(structName,"description is missing its sizeof property.");
      }
      /*if( undefined===si.offset ){
        toss(structName,"description is missing its offset property.");
      }*/
      si.offset ??= 0;
    }else{
      si.offset ??= 0;
    }
    Object.keys(si.members || {}).forEach((k)=>{
      // Sanity checks of sizeof/offset info...
      let m = ads(si.members[k]);
      if(!m.members && !m.sizeof){
        /* ^^^^ fixme: we need to recursively collect all sizeofs
           before updating that. */
        m.sizeof = sigSize(m.signature);
        if(!m.sizeof){
          toss(sPropName(structName,k), "is missing a sizeof property.",m);
        }
      }
      if( undefined===m.offset ){
        if( autoCalc ) m.offset = offset;
        else{
          toss(sPropName(structName,k),"is missing its offset.",
               JSON.stringify(m));
        }
        /* A missing offset on the initial child is okay (it's always
           zero), but we don't know for sure that the members are
           their natural order, so we don't know, at this point, which
           one is "first". */
      }
      si.members[k] = m /* in case ads() resolved it to something else */;
      if(!lastMember || lastMember.offset < m.offset) lastMember = m;
      const oldAutoCalc = !!m.autoCalc;
      if( autoCalc ) m.autoCalcSizeOffset = true;
      makeMemberWrapper.call(self, StructCtor, k, m);
      if( oldAutoCalc ) m.autoCalcSizeOffset = true;
      else delete m.autoCalcSizeOffset;
      offset += m.sizeof;
      //console.warn("offset",sPropName(structName,k),offset);
    });

    if( !lastMember ) toss("No member property descriptions found.");
    if( !si.sizeof ) si.sizeof = offset;
    if(si.sizeof===1){
      (si.signature === 'c' || si.signature === 'C') ||
        toss("Unexpected sizeof==1 member",
             sPropName(structName,k),
             "with signature",si.signature);
    }else{
      // sizes and offsets of size-1 members may be odd values, but
      // others may not.
      if(0!==(si.sizeof%4)){
        console.warn("Invalid struct member description",si);
        toss(structName,"sizeof is not aligned. sizeof="+si.sizeof);
      }
      if(0!==(si.offset%4)){
        console.warn("Invalid struct member description",si);
        toss(structName,"offset is not aligned. offset="+si.offset);
      }
    }
    if( si.sizeof < offset ){
      console.warn("Suspect struct description:",si,"offset =",offset);
      toss("Mismatch in the calculated vs. the provided sizeof/offset info.",
           "Expected sizeof",offset,"but got",si.sizeof,"for",si);
      /* It is legal for the native struct to be larger, so long as
         we're pointing to all the right offsets for the members
         exposed here. */
    }
    delete si.autoCalcSizeOffset;
    return StructCtor;
  }/*StructBinderImpl*/;

  const StructBinder = function StructBinder(structName, structInfo){
    return (1==arguments.length)
      ? StructBinderImpl.call(StructBinder, structName)
      : StructBinderImpl.call(StructBinder, structName, structInfo);
  };
  StructBinder.StructType = StructType;
  StructBinder.config = config;
  StructBinder.allocCString = __allocCString;
  StructBinder.adaptGet = __adaptGet;
  StructBinder.adaptSet = __adaptSet;
  StructBinder.adaptStruct = __adaptStruct;
  StructBinder.ptrAdd = __ptrAdd;
  if(!StructBinder.debugFlags){
    StructBinder.debugFlags = SBF.__makeDebugFlags(SBF.debugFlags);
  }
  return StructBinder;
}/*StructBinderFactory*/;
/*
  2022-07-22

  The author disclaims copyright to this source code.  In place of a
  legal notice, here is a blessing:

  *   May you do good and not evil.
  *   May you find forgiveness for yourself and forgive others.
  *   May you share freely, never taking more than you give.

  ***********************************************************************

  This file glues together disparate pieces of JS which are loaded in
  previous steps of the sqlite3-api.js bootstrapping process:
  sqlite3-api-prologue.js, whwasmutil.js, and jaccwabyt.js. It
  initializes the main API pieces so that the downstream components
  (e.g. sqlite3-api-oo1.js) have all of the infrastructure that they
  need.
*/
globalThis.sqlite3ApiBootstrap.initializers.push(function(sqlite3){
  'use strict';
  const toss = (...args)=>{throw new Error(args.join(' '))};
  const capi = sqlite3.capi, wasm = sqlite3.wasm, util = sqlite3.util;
  globalThis.WhWasmUtilInstaller(wasm);
  delete globalThis.WhWasmUtilInstaller;

  if(0){
    /**
       Please keep this block around as a maintenance reminder
       that we cannot rely on this type of check.

       This block fails on Safari, per a report at
       https://sqlite.org/forum/forumpost/e5b20e1feb.

       It turns out that what Safari serves from the indirect function
       table (e.g. wasm.functionEntry(X)) is anonymous functions which
       wrap the WASM functions, rather than returning the WASM
       functions themselves. That means comparison of such functions
       is useless for determining whether or not we have a specific
       function from wasm.exports. i.e. if function X is indirection
       function table entry N then wasm.exports.X is not equal to
       wasm.functionEntry(N) in Safari, despite being so in the other
       browsers.
    */
    /**
       Find a mapping for SQLITE_WASM_DEALLOC, which the API
       guarantees is a WASM pointer to the same underlying function as
       wasm.dealloc() (noting that wasm.dealloc() is permitted to be a
       JS wrapper around the WASM function). There is unfortunately no
       O(1) algorithm for finding this pointer: we have to walk the
       WASM indirect function table to find it. However, experience
       indicates that that particular function is always very close to
       the front of the table (it's been entry #3 in all relevant
       tests).
    */
    const dealloc = wasm.exports[sqlite3.config.deallocExportName];
    const nFunc = wasm.functionTable().length;
    let i;
    for(i = 0; i < nFunc; ++i){
      const e = wasm.functionEntry(i);
      if(dealloc === e){
        capi.SQLITE_WASM_DEALLOC = i;
        break;
      }
    }
    if(dealloc !== wasm.functionEntry(capi.SQLITE_WASM_DEALLOC)){
      toss("Internal error: cannot find function pointer for SQLITE_WASM_DEALLOC.");
    }
  }

  /**
     Signatures for the WASM-exported C-side functions. Each entry
     is an array with 2+ elements:

     [ "c-side name",
       "result type" (wasm.xWrap() syntax),
       [arg types in xWrap() syntax]
       // ^^^ this needn't strictly be an array: it can be subsequent
       // elements instead: [x,y,z] is equivalent to x,y,z
     ]

     Support for the API-specific data types in the result/argument
     type strings gets plugged in at a later phase in the API
     initialization process.
  */
  const bindingSignatures = {
    core: [
      // Please keep these sorted by function name!
      ["sqlite3_aggregate_context","void*", "sqlite3_context*", "int"],
      /* sqlite3_auto_extension() has a hand-written binding. */
      /* sqlite3_bind_blob() and sqlite3_bind_text() have hand-written
         bindings to permit more flexible inputs. */
      ["sqlite3_bind_double","int", "sqlite3_stmt*", "int", "f64"],
      ["sqlite3_bind_int","int", "sqlite3_stmt*", "int", "int"],
      ["sqlite3_bind_null",undefined, "sqlite3_stmt*", "int"],
      ["sqlite3_bind_parameter_count", "int", "sqlite3_stmt*"],
      ["sqlite3_bind_parameter_index","int", "sqlite3_stmt*", "string"],
      ["sqlite3_bind_parameter_name", "string", "sqlite3_stmt*", "int"],
      ["sqlite3_bind_pointer", "int",
       "sqlite3_stmt*", "int", "*", "string:static", "*"],
      /* sqlite_bind_text() is hand-written */
      ["sqlite3_bind_zeroblob", "int", "sqlite3_stmt*", "int", "int"],
      ["sqlite3_busy_handler","int", [
        "sqlite3*",
        new wasm.xWrap.FuncPtrAdapter({
          signature: 'i(pi)',
          contextKey: (argv,argIndex)=>argv[0/* sqlite3* */]
        }),
        "*"
      ]],
      ["sqlite3_busy_timeout","int", "sqlite3*", "int"],
      /* sqlite3_cancel_auto_extension() has a hand-written binding. */
      /* sqlite3_close_v2() is implemented by hand to perform some
         extra work. */
      ["sqlite3_changes", "int", "sqlite3*"],
      ["sqlite3_clear_bindings","int", "sqlite3_stmt*"],
      ["sqlite3_collation_needed", "int", "sqlite3*", "*", "*"/*=>v(ppis)*/],
      ["sqlite3_column_blob","*", "sqlite3_stmt*", "int"],
      ["sqlite3_column_bytes","int", "sqlite3_stmt*", "int"],
      ["sqlite3_column_count", "int", "sqlite3_stmt*"],
      ["sqlite3_column_decltype", "string", "sqlite3_stmt*", "int"],
      ["sqlite3_column_double","f64", "sqlite3_stmt*", "int"],
      ["sqlite3_column_int","int", "sqlite3_stmt*", "int"],
      ["sqlite3_column_name","string", "sqlite3_stmt*", "int"],
      ["sqlite3_column_type","int", "sqlite3_stmt*", "int"],
      ["sqlite3_column_value","sqlite3_value*", "sqlite3_stmt*", "int"],
      ["sqlite3_commit_hook", "void*", [
        "sqlite3*",
        new wasm.xWrap.FuncPtrAdapter({
          name: 'sqlite3_commit_hook',
          signature: 'i(p)',
          contextKey: (argv)=>argv[0/* sqlite3* */]
        }),
        '*'
      ]],
      ["sqlite3_compileoption_get", "string", "int"],
      ["sqlite3_compileoption_used", "int", "string"],
      ["sqlite3_complete", "int", "string:flexible"],
      ["sqlite3_context_db_handle", "sqlite3*", "sqlite3_context*"],
      /* sqlite3_create_collation() and sqlite3_create_collation_v2()
         use hand-written bindings to simplify passing of the callback
         function. */
      /* sqlite3_create_function(), sqlite3_create_function_v2(), and
         sqlite3_create_window_function() use hand-written bindings to
         simplify handling of their function-type arguments. */
      ["sqlite3_data_count", "int", "sqlite3_stmt*"],
      ["sqlite3_db_filename", "string", "sqlite3*", "string"],
      ["sqlite3_db_handle", "sqlite3*", "sqlite3_stmt*"],
      ["sqlite3_db_name", "string", "sqlite3*", "int"],
      ["sqlite3_db_readonly", "int", "sqlite3*", "string"],
      ["sqlite3_db_status", "int", "sqlite3*", "int", "*", "*", "int"],
      ["sqlite3_errcode", "int", "sqlite3*"],
      ["sqlite3_errmsg", "string", "sqlite3*"],
      ["sqlite3_error_offset", "int", "sqlite3*"],
      ["sqlite3_errstr", "string", "int"],
      ["sqlite3_exec", "int", [
        "sqlite3*", "string:flexible",
        new wasm.xWrap.FuncPtrAdapter({
          signature: 'i(pipp)',
          bindScope: 'transient',
          callProxy: (callback)=>{
            let aNames;
            return (pVoid, nCols, pColVals, pColNames)=>{
              try {
                const aVals = wasm.cArgvToJs(nCols, pColVals);
                if(!aNames) aNames = wasm.cArgvToJs(nCols, pColNames);
                return callback(aVals, aNames) | 0;
              }catch(e){
                /* If we set the db error state here, the higher-level
                   exec() call replaces it with its own, so we have no way
                   of reporting the exception message except the console. We
                   must not propagate exceptions through the C API. Though
                   we make an effort to report OOM here, sqlite3_exec()
                   translates that into SQLITE_ABORT as well. */
                return e.resultCode || capi.SQLITE_ERROR;
              }
            }
          }
        }),
        "*", "**"
      ]],
      ["sqlite3_expanded_sql", "string", "sqlite3_stmt*"],
      ["sqlite3_extended_errcode", "int", "sqlite3*"],
      ["sqlite3_extended_result_codes", "int", "sqlite3*", "int"],
      ["sqlite3_file_control", "int", "sqlite3*", "string", "int", "*"],
      ["sqlite3_finalize", "int", "sqlite3_stmt*"],
      ["sqlite3_free", undefined,"*"],
      ["sqlite3_get_autocommit", "int", "sqlite3*"],
      ["sqlite3_get_auxdata", "*", "sqlite3_context*", "int"],
      ["sqlite3_initialize", undefined],
      ["sqlite3_interrupt", undefined, "sqlite3*"],
      ["sqlite3_is_interrupted", "int", "sqlite3*"],
      ["sqlite3_keyword_count", "int"],
      ["sqlite3_keyword_name", "int", ["int", "**", "*"]],
      ["sqlite3_keyword_check", "int", ["string", "int"]],
      ["sqlite3_libversion", "string"],
      ["sqlite3_libversion_number", "int"],
      ["sqlite3_limit", "int", ["sqlite3*", "int", "int"]],
      ["sqlite3_malloc", "*","int"],
      ["sqlite3_next_stmt", "sqlite3_stmt*", ["sqlite3*","sqlite3_stmt*"]],
      ["sqlite3_open", "int", "string", "*"],
      ["sqlite3_open_v2", "int", "string", "*", "int", "string"],
      /* sqlite3_prepare_v2() and sqlite3_prepare_v3() are handled
         separately due to us requiring two different sets of semantics
         for those, depending on how their SQL argument is provided. */
      /* sqlite3_randomness() uses a hand-written wrapper to extend
         the range of supported argument types. */
      ["sqlite3_realloc", "*","*","int"],
      ["sqlite3_reset", "int", "sqlite3_stmt*"],
      /* sqlite3_reset_auto_extension() has a hand-written binding. */
      ["sqlite3_result_blob", undefined, "sqlite3_context*", "*", "int", "*"],
      ["sqlite3_result_double", undefined, "sqlite3_context*", "f64"],
      ["sqlite3_result_error", undefined, "sqlite3_context*", "string", "int"],
      ["sqlite3_result_error_code", undefined, "sqlite3_context*", "int"],
      ["sqlite3_result_error_nomem", undefined, "sqlite3_context*"],
      ["sqlite3_result_error_toobig", undefined, "sqlite3_context*"],
      ["sqlite3_result_int", undefined, "sqlite3_context*", "int"],
      ["sqlite3_result_null", undefined, "sqlite3_context*"],
      ["sqlite3_result_pointer", undefined,
       "sqlite3_context*", "*", "string:static", "*"],
      ["sqlite3_result_subtype", undefined, "sqlite3_value*", "int"],
      ["sqlite3_result_text", undefined, "sqlite3_context*", "string", "int", "*"],
      ["sqlite3_result_zeroblob", undefined, "sqlite3_context*", "int"],
      ["sqlite3_rollback_hook", "void*", [
        "sqlite3*",
        new wasm.xWrap.FuncPtrAdapter({
          name: 'sqlite3_rollback_hook',
          signature: 'v(p)',
          contextKey: (argv)=>argv[0/* sqlite3* */]
        }),
        '*'
      ]],
      /**
         We do not have a way to automatically clean up destructors
         which are automatically converted from JS functions via the
         final argument to sqlite3_set_auxdata(). Because of that,
         automatic function conversion is not supported for this
         function.  Clients should use wasm.installFunction() to create
         such callbacks, then pass that pointer to
         sqlite3_set_auxdata(). Relying on automated conversions here
         would lead to leaks of JS/WASM proxy functions because
         sqlite3_set_auxdata() is frequently called in UDFs.

         The sqlite3.oo1.DB class's onclose handlers can be used for this
         purpose. For example:

         const pAuxDtor = wasm.installFunction('v(p)', function(ptr){
           //free ptr
         });
         myDb.onclose = {
           after: ()=>{
             wasm.uninstallFunction(pAuxDtor);
           }
         };

         Then pass pAuxDtor as the final argument to appropriate
         sqlite3_set_auxdata() calls.

         Prior to 3.49.0 this binding ostensibly had automatic
         function conversion here but a typo prevented it from
         working.  Rather than fix it, it was removed because testing
         the fix brought the huge potential for memory leaks to the
         forefront.
      */
      ["sqlite3_set_auxdata", undefined, [
        "sqlite3_context*", "int", "*",
        true
          ? "*"
          : new wasm.xWrap.FuncPtrAdapter({
            /* If we can find a way to automate their cleanup, JS functions can
               be auto-converted with this. */
            name: 'xDestroyAuxData',
            signature: 'v(p)',
            contextKey: (argv, argIndex)=>argv[0/* sqlite3_context* */]
          })
      ]],
      ['sqlite3_set_errmsg', 'int', 'sqlite3*', 'int', 'string'],
      ["sqlite3_shutdown", undefined],
      ["sqlite3_sourceid", "string"],
      ["sqlite3_sql", "string", "sqlite3_stmt*"],
      ["sqlite3_status", "int", "int", "*", "*", "int"],
      ["sqlite3_step", "int", "sqlite3_stmt*"],
      ["sqlite3_stmt_busy", "int", "sqlite3_stmt*"],
      ["sqlite3_stmt_readonly", "int", "sqlite3_stmt*"],
      ["sqlite3_stmt_status", "int", "sqlite3_stmt*", "int", "int"],
      ["sqlite3_strglob", "int", "string","string"],
      ["sqlite3_stricmp", "int", "string", "string"],
      ["sqlite3_strlike", "int", "string", "string","int"],
      ["sqlite3_strnicmp", "int", "string", "string", "int"],
      ["sqlite3_table_column_metadata", "int",
       "sqlite3*", "string", "string", "string",
       "**", "**", "*", "*", "*"],
      ["sqlite3_total_changes", "int", "sqlite3*"],
      ["sqlite3_trace_v2", "int", [
        "sqlite3*", "int",
        new wasm.xWrap.FuncPtrAdapter({
          name: 'sqlite3_trace_v2::callback',
          signature: 'i(ippp)',
          contextKey: (argv,argIndex)=>argv[0/* sqlite3* */]
        }),
        "*"
      ]],
      ["sqlite3_txn_state", "int", ["sqlite3*","string"]],
      /* sqlite3_uri_...() have very specific requirements for their
         first C-string arguments, so we cannot perform any
         string-type conversion on their first argument. */
      ["sqlite3_uri_boolean", "int", "sqlite3_filename", "string", "int"],
      ["sqlite3_uri_key", "string", "sqlite3_filename", "int"],
      ["sqlite3_uri_parameter", "string", "sqlite3_filename", "string"],
      ["sqlite3_user_data","void*", "sqlite3_context*"],
      ["sqlite3_value_blob", "*", "sqlite3_value*"],
      ["sqlite3_value_bytes","int", "sqlite3_value*"],
      ["sqlite3_value_double","f64", "sqlite3_value*"],
      ["sqlite3_value_dup", "sqlite3_value*", "sqlite3_value*"],
      ["sqlite3_value_free", undefined, "sqlite3_value*"],
      ["sqlite3_value_frombind", "int", "sqlite3_value*"],
      ["sqlite3_value_int","int", "sqlite3_value*"],
      ["sqlite3_value_nochange", "int", "sqlite3_value*"],
      ["sqlite3_value_numeric_type", "int", "sqlite3_value*"],
      ["sqlite3_value_pointer", "*", "sqlite3_value*", "string:static"],
      ["sqlite3_value_subtype", "int", "sqlite3_value*"],
      ["sqlite3_value_type", "int", "sqlite3_value*"],
      ["sqlite3_vfs_find", "*", "string"],
      ["sqlite3_vfs_register", "int", "sqlite3_vfs*", "int"],
      ["sqlite3_vfs_unregister", "int", "sqlite3_vfs*"]

      /* This list gets extended with optional pieces below */
    ]/*.core*/,
    /**
       Functions which require BigInt (int64) support are separated
       from the others because we need to conditionally bind them or
       apply dummy impls, depending on the capabilities of the
       environment.  (That said: we never actually build without
       BigInt support, and such builds are untested.)

       Not all of these functions directly require int64 but are only
       for use with APIs which require int64. For example, the
       vtab-related functions.
    */
    int64: [
      ["sqlite3_bind_int64","int", ["sqlite3_stmt*", "int", "i64"]],
      ["sqlite3_changes64","i64", ["sqlite3*"]],
      ["sqlite3_column_int64","i64", ["sqlite3_stmt*", "int"]],
      ["sqlite3_deserialize", "int", "sqlite3*", "string", "*", "i64", "i64", "int"]
      /* Careful! Short version: de/serialize() are problematic because they
         might use a different allocator than the user for managing the
         deserialized block. de/serialize() are ONLY safe to use with
         sqlite3_malloc(), sqlite3_free(), and its 64-bit variants. Because
         of this, the canonical builds of sqlite3.wasm/js guarantee that
         sqlite3.wasm.alloc() and friends use those allocators. Custom builds
         may not guarantee that, however. */,
      ["sqlite3_last_insert_rowid", "i64", ["sqlite3*"]],
      ["sqlite3_malloc64", "*","i64"],
      ["sqlite3_msize", "i64", "*"],
      ["sqlite3_overload_function", "int", ["sqlite3*","string","int"]],
      ["sqlite3_realloc64", "*","*", "i64"],
      ["sqlite3_result_int64", undefined, "*", "i64"],
      ["sqlite3_result_zeroblob64", "int", "*", "i64"],
      ["sqlite3_serialize","*", "sqlite3*", "string", "*", "int"],
      ["sqlite3_set_last_insert_rowid", undefined, ["sqlite3*", "i64"]],
      ["sqlite3_status64", "int", "int", "*", "*", "int"],
      ["sqlite3_db_status64", "int", "sqlite3*", "int", "*", "*", "int"],
      ["sqlite3_total_changes64", "i64", ["sqlite3*"]],
      ["sqlite3_update_hook", "*", [
        "sqlite3*",
        new wasm.xWrap.FuncPtrAdapter({
          name: 'sqlite3_update_hook::callback',
          signature: "v(pippj)",
          contextKey: (argv)=>argv[0/* sqlite3* */],
          callProxy: (callback)=>{
            return (p,op,z0,z1,rowid)=>{
              callback(p, op, wasm.cstrToJs(z0), wasm.cstrToJs(z1), rowid);
            };
          }
        }),
        "*"
      ]],
      ["sqlite3_uri_int64", "i64", ["sqlite3_filename", "string", "i64"]],
      ["sqlite3_value_int64","i64", "sqlite3_value*"]
      /* This list gets extended with optional pieces below */
    ]/*.int64*/,
    /**
       Functions which are intended solely for API-internal use by the
       WASM components, not client code. These get installed into
       sqlite3.util. Some of them get exposed to clients via variants
       in sqlite3_js_...().

       2024-01-11: these were renamed, with two underscores in the
       prefix, to ensure that clients do not accidentally depend on
       them.  They have always been documented as internal-use-only,
       so no clients "should" be depending on the old names.
    */
    wasmInternal: [
      ["sqlite3__wasm_db_reset", "int", "sqlite3*"],
      ["sqlite3__wasm_db_vfs", "sqlite3_vfs*", "sqlite3*","string"],
      [/* DO NOT USE. This is deprecated since 2023-08-11 because it
          can trigger assert() in debug builds when used with file
          sizes which are not an exact multiple of a valid db page
          size.  This function is retained only so that
          sqlite3_js_vfs_create_file() can continue to work (for a
          given value of work), but that function emits a
          config.warn() log message directing the reader to
          alternatives. */
        "sqlite3__wasm_vfs_create_file", "int", "sqlite3_vfs*","string","*", "int"
      ],
      ["sqlite3__wasm_posix_create_file", "int", "string","*", "int"],
      ["sqlite3__wasm_vfs_unlink", "int", "sqlite3_vfs*","string"],
      ["sqlite3__wasm_qfmt_token","string:dealloc", "string","int"]
    ]/*.wasmInternal*/
  } /*bindingSignatures*/;

  if( !!wasm.exports.sqlite3_progress_handler ){
    bindingSignatures.core.push(
      ["sqlite3_progress_handler", undefined, [
        "sqlite3*", "int", new wasm.xWrap.FuncPtrAdapter({
          name: 'xProgressHandler',
          signature: 'i(p)',
          bindScope: 'context',
          contextKey: (argv,argIndex)=>argv[0/* sqlite3* */]
        }), "*"
      ]]
    );
  }

  if( !!wasm.exports.sqlite3_stmt_explain ){
    bindingSignatures.core.push(
      ["sqlite3_stmt_explain", "int", "sqlite3_stmt*", "int"],
      ["sqlite3_stmt_isexplain", "int", "sqlite3_stmt*"]
    );
  }

  if( !!wasm.exports.sqlite3_set_authorizer ){
    bindingSignatures.core.push(
      ["sqlite3_set_authorizer", "int", [
        "sqlite3*",
        new wasm.xWrap.FuncPtrAdapter({
          name: "sqlite3_set_authorizer::xAuth",
          signature: "i(pi"+"ssss)",
          contextKey: (argv, argIndex)=>argv[0/*(sqlite3*)*/],
          /**
             We use callProxy here to ensure (A) that exceptions
             thrown from callback() have well-defined behavior and (B)
             that its result is coerced to an integer.
          */
          callProxy: (callback)=>{
            return (pV, iCode, s0, s1, s2, s3)=>{
              try{
                s0 = s0 && wasm.cstrToJs(s0); s1 = s1 && wasm.cstrToJs(s1);
                s2 = s2 && wasm.cstrToJs(s2); s3 = s3 && wasm.cstrToJs(s3);
                return callback(pV, iCode, s0, s1, s2, s3) | 0;
              }catch(e){
                return e.resultCode || capi.SQLITE_ERROR;
              }
            }
          }
        }),
        "*"/*pUserData*/
      ]]
    );
  }/* sqlite3_set_authorizer() */

  if( !!wasm.exports.sqlite3_column_origin_name ){
    bindingSignatures.core.push(
      ["sqlite3_column_database_name","string", "sqlite3_stmt*", "int"],
      ["sqlite3_column_origin_name","string", "sqlite3_stmt*", "int"],
      ["sqlite3_column_table_name","string", "sqlite3_stmt*", "int"]
    );
  }

  if(false && wasm.compileOptionUsed('SQLITE_ENABLE_NORMALIZE')){
    /* ^^^ "the problem" is that this is an optional feature and the
       build-time function-export list does not currently take
       optional features into account. */
    bindingSignatures.core.push(["sqlite3_normalized_sql", "string", "sqlite3_stmt*"]);
  }


  if( wasm.bigIntEnabled && !!wasm.exports.sqlite3_declare_vtab ){
    bindingSignatures.int64.push(
      ["sqlite3_create_module", "int",
       ["sqlite3*","string","sqlite3_module*","*"]],
      ["sqlite3_create_module_v2", "int",
       ["sqlite3*","string","sqlite3_module*","*","*"]],
      ["sqlite3_declare_vtab", "int", ["sqlite3*", "string:flexible"]],
      ["sqlite3_drop_modules", "int", ["sqlite3*", "**"]],
      ["sqlite3_vtab_collation","string","sqlite3_index_info*","int"],
      /*["sqlite3_vtab_config" is variadic and requires a hand-written
        proxy.] */
      ["sqlite3_vtab_distinct","int", "sqlite3_index_info*"],
      ["sqlite3_vtab_in","int", "sqlite3_index_info*", "int", "int"],
      ["sqlite3_vtab_in_first", "int", "sqlite3_value*", "**"],
      ["sqlite3_vtab_in_next", "int", "sqlite3_value*", "**"],
      ["sqlite3_vtab_nochange","int", "sqlite3_context*"],
      ["sqlite3_vtab_on_conflict","int", "sqlite3*"],
      ["sqlite3_vtab_rhs_value","int", "sqlite3_index_info*", "int", "**"]
    );
  }/* virtual table APIs */

  if(wasm.bigIntEnabled && !!wasm.exports.sqlite3_preupdate_hook){
    bindingSignatures.int64.push(
      ["sqlite3_preupdate_blobwrite", "int", "sqlite3*"],
      ["sqlite3_preupdate_count", "int", "sqlite3*"],
      ["sqlite3_preupdate_depth", "int", "sqlite3*"],
      ["sqlite3_preupdate_hook", "*", [
        "sqlite3*",
        new wasm.xWrap.FuncPtrAdapter({
          name: 'sqlite3_preupdate_hook',
          signature: "v(ppippjj)",
          contextKey: (argv)=>argv[0/* sqlite3* */],
          callProxy: (callback)=>{
            return (p,db,op,zDb,zTbl,iKey1,iKey2)=>{
              callback(p, db, op, wasm.cstrToJs(zDb), wasm.cstrToJs(zTbl),
                       iKey1, iKey2);
            };
          }
        }),
        "*"
      ]],
      ["sqlite3_preupdate_new", "int", ["sqlite3*", "int", "**"]],
      ["sqlite3_preupdate_old", "int", ["sqlite3*", "int", "**"]]
    );
  } /* preupdate API */

  // Add session/changeset APIs...
  if(wasm.bigIntEnabled
     && !!wasm.exports.sqlite3changegroup_add
     && !!wasm.exports.sqlite3session_create
     && !!wasm.exports.sqlite3_preupdate_hook /* required by the session API */){
    /**
       FuncPtrAdapter options for session-related callbacks with the
       native signature "i(ps)". This proxy converts the 2nd argument
       from a C string to a JS string before passing the arguments on
       to the client-provided JS callback.
    */
    const __ipsProxy = {
      signature: 'i(ps)',
      callProxy:(callback)=>{
        return (p,s)=>{
          try{return callback(p, wasm.cstrToJs(s)) | 0}
          catch(e){return e.resultCode || capi.SQLITE_ERROR}
        }
      }
    };

    bindingSignatures.int64.push(
      ['sqlite3changegroup_add', 'int', ['sqlite3_changegroup*', 'int', 'void*']],
      ['sqlite3changegroup_add_strm', 'int', [
        'sqlite3_changegroup*',
        new wasm.xWrap.FuncPtrAdapter({
          name: 'xInput', signature: 'i(ppp)', bindScope: 'transient'
        }),
        'void*'
      ]],
      ['sqlite3changegroup_delete', undefined, ['sqlite3_changegroup*']],
      ['sqlite3changegroup_new', 'int', ['**']],
      ['sqlite3changegroup_output', 'int', ['sqlite3_changegroup*', 'int*', '**']],
      ['sqlite3changegroup_output_strm', 'int', [
        'sqlite3_changegroup*',
        new wasm.xWrap.FuncPtrAdapter({
          name: 'xOutput', signature: 'i(ppi)', bindScope: 'transient'
        }),
        'void*'
      ]],
      ['sqlite3changeset_apply', 'int', [
        'sqlite3*', 'int', 'void*',
        new wasm.xWrap.FuncPtrAdapter({
          name: 'xFilter', bindScope: 'transient', ...__ipsProxy
        }),
        new wasm.xWrap.FuncPtrAdapter({
          name: 'xConflict', signature: 'i(pip)', bindScope: 'transient'
        }),
        'void*'
      ]],
      ['sqlite3changeset_apply_strm', 'int', [
        'sqlite3*',
        new wasm.xWrap.FuncPtrAdapter({
          name: 'xInput', signature: 'i(ppp)', bindScope: 'transient'
        }),
        'void*',
        new wasm.xWrap.FuncPtrAdapter({
          name: 'xFilter', bindScope: 'transient', ...__ipsProxy
        }),
        new wasm.xWrap.FuncPtrAdapter({
          name: 'xConflict', signature: 'i(pip)', bindScope: 'transient'
        }),
        'void*'
      ]],
      ['sqlite3changeset_apply_v2', 'int', [
        'sqlite3*', 'int', 'void*',
        new wasm.xWrap.FuncPtrAdapter({
          name: 'xFilter', bindScope: 'transient', ...__ipsProxy
        }),
        new wasm.xWrap.FuncPtrAdapter({
          name: 'xConflict', signature: 'i(pip)', bindScope: 'transient'
        }),
        'void*', '**', 'int*', 'int'

      ]],
      ['sqlite3changeset_apply_v2_strm', 'int', [
        'sqlite3*',
        new wasm.xWrap.FuncPtrAdapter({
          name: 'xInput', signature: 'i(ppp)', bindScope: 'transient'
        }),
        'void*',
        new wasm.xWrap.FuncPtrAdapter({
          name: 'xFilter', bindScope: 'transient', ...__ipsProxy
        }),
        new wasm.xWrap.FuncPtrAdapter({
          name: 'xConflict', signature: 'i(pip)', bindScope: 'transient'
        }),
        'void*', '**', 'int*', 'int'
      ]],
      ['sqlite3changeset_apply_v3', 'int', [
        'sqlite3*', 'int', 'void*',
        new wasm.xWrap.FuncPtrAdapter({
          name: 'xFilter', signature: 'i(pp)', bindScope: 'transient'
        }),
        new wasm.xWrap.FuncPtrAdapter({
          name: 'xConflict', signature: 'i(pip)', bindScope: 'transient'
        }),
        'void*', '**', 'int*', 'int'

      ]],
      ['sqlite3changeset_apply_v3_strm', 'int', [
        'sqlite3*',
        new wasm.xWrap.FuncPtrAdapter({
          name: 'xInput', signature: 'i(ppp)', bindScope: 'transient'
        }),
        'void*',
        new wasm.xWrap.FuncPtrAdapter({
          name: 'xFilter', signature: 'i(pp)', bindScope: 'transient'
        }),
        new wasm.xWrap.FuncPtrAdapter({
          name: 'xConflict', signature: 'i(pip)', bindScope: 'transient'
        }),
        'void*', '**', 'int*', 'int'
      ]],
      ['sqlite3changeset_concat', 'int', ['int','void*', 'int', 'void*', 'int*', '**']],
      ['sqlite3changeset_concat_strm', 'int', [
        new wasm.xWrap.FuncPtrAdapter({
          name: 'xInputA', signature: 'i(ppp)', bindScope: 'transient'
        }),
        'void*',
        new wasm.xWrap.FuncPtrAdapter({
          name: 'xInputB', signature: 'i(ppp)', bindScope: 'transient'
        }),
        'void*',
        new wasm.xWrap.FuncPtrAdapter({
          name: 'xOutput', signature: 'i(ppi)', bindScope: 'transient'
        }),
        'void*'
      ]],
      ['sqlite3changeset_conflict', 'int', ['sqlite3_changeset_iter*', 'int', '**']],
      ['sqlite3changeset_finalize', 'int', ['sqlite3_changeset_iter*']],
      ['sqlite3changeset_fk_conflicts', 'int', ['sqlite3_changeset_iter*', 'int*']],
      ['sqlite3changeset_invert', 'int', ['int', 'void*', 'int*', '**']],
      ['sqlite3changeset_invert_strm', 'int', [
        new wasm.xWrap.FuncPtrAdapter({
          name: 'xInput', signature: 'i(ppp)', bindScope: 'transient'
        }),
        'void*',
        new wasm.xWrap.FuncPtrAdapter({
          name: 'xOutput', signature: 'i(ppi)', bindScope: 'transient'
        }),
        'void*'
      ]],
      ['sqlite3changeset_new', 'int', ['sqlite3_changeset_iter*', 'int', '**']],
      ['sqlite3changeset_next', 'int', ['sqlite3_changeset_iter*']],
      ['sqlite3changeset_old', 'int', ['sqlite3_changeset_iter*', 'int', '**']],
      ['sqlite3changeset_op', 'int', [
        'sqlite3_changeset_iter*', '**', 'int*', 'int*','int*'
      ]],
      ['sqlite3changeset_pk', 'int', ['sqlite3_changeset_iter*', '**', 'int*']],
      ['sqlite3changeset_start', 'int', ['**', 'int', '*']],
      ['sqlite3changeset_start_strm', 'int', [
        '**',
        new wasm.xWrap.FuncPtrAdapter({
          name: 'xInput', signature: 'i(ppp)', bindScope: 'transient'
        }),
        'void*'
      ]],
      ['sqlite3changeset_start_v2', 'int', ['**', 'int', '*', 'int']],
      ['sqlite3changeset_start_v2_strm', 'int', [
        '**',
        new wasm.xWrap.FuncPtrAdapter({
          name: 'xInput', signature: 'i(ppp)', bindScope: 'transient'
        }),
        'void*', 'int'
      ]],
      ['sqlite3session_attach', 'int', ['sqlite3_session*', 'string']],
      ['sqlite3session_changeset', 'int', ['sqlite3_session*', 'int*', '**']],
      ['sqlite3session_changeset_size', 'i64', ['sqlite3_session*']],
      ['sqlite3session_changeset_strm', 'int', [
        'sqlite3_session*',
        new wasm.xWrap.FuncPtrAdapter({
          name: 'xOutput', signature: 'i(ppp)', bindScope: 'transient'
        }),
        'void*'
      ]],
      ['sqlite3session_config', 'int', ['int', 'void*']],
      ['sqlite3session_create', 'int', ['sqlite3*', 'string', '**']],
      //sqlite3session_delete() is bound manually
      ['sqlite3session_diff', 'int', ['sqlite3_session*', 'string', 'string', '**']],
      ['sqlite3session_enable', 'int', ['sqlite3_session*', 'int']],
      ['sqlite3session_indirect', 'int', ['sqlite3_session*', 'int']],
      ['sqlite3session_isempty', 'int', ['sqlite3_session*']],
      ['sqlite3session_memory_used', 'i64', ['sqlite3_session*']],
      ['sqlite3session_object_config', 'int', ['sqlite3_session*', 'int', 'void*']],
      ['sqlite3session_patchset', 'int', ['sqlite3_session*', '*', '**']],
      ['sqlite3session_patchset_strm', 'int', [
        'sqlite3_session*',
        new wasm.xWrap.FuncPtrAdapter({
          name: 'xOutput', signature: 'i(ppp)', bindScope: 'transient'
        }),
        'void*'
      ]],
      ['sqlite3session_table_filter', undefined, [
        'sqlite3_session*',
        new wasm.xWrap.FuncPtrAdapter({
          name: 'xFilter', ...__ipsProxy,
          contextKey: (argv,argIndex)=>argv[0/* (sqlite3_session*) */]
        }),
        '*'
      ]]
    );
  }/*session/changeset APIs*/

  /**
     Prepare JS<->C struct bindings for the non-opaque struct types we
     need...
  */
  sqlite3.StructBinder = globalThis.Jaccwabyt({
    heap: wasm.heap8u,
    alloc: wasm.alloc,
    dealloc: wasm.dealloc,
    bigIntEnabled: wasm.bigIntEnabled,
    pointerIR: wasm.ptr.ir,
    memberPrefix: /* Never change this: this prefix is baked into any
                     amount of code and client-facing docs. (Much
                     later: it probably should have been '$$', but see
                     the previous sentence.) */ '$'
  });
  delete globalThis.Jaccwabyt;

  {// wasm.xWrap() bindings...

    /* Convert Arrays and certain TypedArrays to strings for
       'string:flexible'-type arguments */
    const __xString = wasm.xWrap.argAdapter('string');
    wasm.xWrap.argAdapter(
      'string:flexible', (v)=>__xString(util.flexibleString(v))
    );

    /**
       The 'string:static' argument adapter treats its argument as
       either...

       - WASM pointer: assumed to be a long-lived C-string which gets
         returned as-is.

       - Anything else: gets coerced to a JS string for use as a map
         key. If a matching entry is found (as described next), it is
         returned, else wasm.allocCString() is used to create a a new
         string, map its pointer to a copy of (''+v) for the remainder
         of the application's life, and returns that pointer value for
         this call and all future calls which are passed a
         string-equivalent argument.

       Use case: sqlite3_bind_pointer(), sqlite3_result_pointer(), and
       sqlite3_value_pointer() call for "a static string and
       preferably a string literal". This converter is used to ensure
       that the string value seen by those functions is long-lived and
       behaves as they need it to, at the cost of a one-time leak of
       each distinct key.
    */
    wasm.xWrap.argAdapter(
      'string:static',
      function(v){
        if(wasm.isPtr(v)) return v;
        v = ''+v;
        let rc = this[v];
        return rc || (this[v] = wasm.allocCString(v));
      }.bind(Object.create(null))
    );

    /**
       Add some descriptive xWrap() aliases for '*' intended to (A)
       improve readability/correctness of bindingSignatures and (B)
       provide automatic conversion from higher-level representations,
       e.g. capi.sqlite3_vfs to `sqlite3_vfs*` via (capi.sqlite3_vfs
       instance).pointer.
    */
    const __xArgPtr = wasm.xWrap.argAdapter('*');
    const nilType = function(){
      /*a class which no value can ever be an instance of*/
    };
    wasm.xWrap.argAdapter('sqlite3_filename', __xArgPtr)
    ('sqlite3_context*', __xArgPtr)
    ('sqlite3_value*', __xArgPtr)
    ('void*', __xArgPtr)
    ('sqlite3_changegroup*', __xArgPtr)
    ('sqlite3_changeset_iter*', __xArgPtr)
    ('sqlite3_session*', __xArgPtr)
    ('sqlite3_stmt*', (v)=>
      __xArgPtr((v instanceof (sqlite3?.oo1?.Stmt || nilType))
           ? v.pointer : v))
    ('sqlite3*', (v)=>
      __xArgPtr((v instanceof (sqlite3?.oo1?.DB || nilType))
           ? v.pointer : v))
    /**
       `sqlite3_vfs*`:

       - v is-a string: use the result of sqlite3_vfs_find(v) but
         throw if it returns 0.
       - v is-a capi.sqlite3_vfs: use v.pointer.
       - Else return the same as the `'*'` argument conversion.
    */
    ('sqlite3_vfs*', (v)=>{
      if('string'===typeof v){
        /* A NULL sqlite3_vfs pointer will be treated as the default
           VFS in many contexts. We specifically do not want that
           behavior here. */
        return capi.sqlite3_vfs_find(v)
          || sqlite3.SQLite3Error.toss(
            capi.SQLITE_NOTFOUND,
            "Unknown sqlite3_vfs name:", v
          );
      }
      return __xArgPtr((v instanceof (capi.sqlite3_vfs || nilType))
                       ? v.pointer : v);
    });
    if( wasm.exports.sqlite3_declare_vtab ){
      wasm.xWrap.argAdapter('sqlite3_index_info*', (v)=>
        __xArgPtr((v instanceof (capi.sqlite3_index_info || nilType))
                  ? v.pointer : v))
      ('sqlite3_module*', (v)=>
        __xArgPtr((v instanceof (capi.sqlite3_module || nilType))
                  ? v.pointer : v)
      );
    }

    /**
       Alias `T*` to `*` for return type conversions for common T
       types, primarily to improve legibility of their binding
       signatures.
    */
    const __xRcPtr = wasm.xWrap.resultAdapter('*');
    wasm.xWrap.resultAdapter('sqlite3*', __xRcPtr)
    ('sqlite3_context*', __xRcPtr)
    ('sqlite3_stmt*', __xRcPtr)
    ('sqlite3_value*', __xRcPtr)
    ('sqlite3_vfs*', __xRcPtr)
    ('void*', __xRcPtr);

    /**
       Populate api object with sqlite3_...() by binding the "raw" wasm
       exports into type-converting proxies using wasm.xWrap().
    */
    for(const e of bindingSignatures.core){
      capi[e[0]] = wasm.xWrap.apply(null, e);
    }
    for(const e of bindingSignatures.wasmInternal){
      util[e[0]] = wasm.xWrap.apply(null, e);
    }

    /* For C API functions which cannot work properly unless
       wasm.bigIntEnabled is true, install a bogus impl which throws
       if called when bigIntEnabled is false. The alternative would be
       to elide these functions altogether, which seems likely to
       cause more confusion, as documented APIs would resolve to the
       undefined value in incompatible builds. */
    for(const e of bindingSignatures.int64){
      capi[e[0]] = wasm.bigIntEnabled
        ? wasm.xWrap.apply(null, e)
        : ()=>toss(e[0]+"() is unavailable due to lack",
                   "of BigInt support in this build.");
    }

    /* We're done with bindingSignatures but it's const, so... */
    delete bindingSignatures.core;
    delete bindingSignatures.int64;
    delete bindingSignatures.wasmInternal;

    /**
       Sets the given db's error state. Accepts:

       - (sqlite3*, int code, string msg)
       - (sqlite3*, Error e [,string msg = ''+e])

       If passed a WasmAllocError, the message is ignored and the
       result code is SQLITE_NOMEM. If passed any other Error type,
       the result code defaults to SQLITE_ERROR unless the Error
       object has a resultCode property, in which case that is used
       (e.g. SQLite3Error has that). If passed a non-WasmAllocError
       exception, the message string defaults to ''+theError.

       Returns either the final result code, capi.SQLITE_NOMEM if
       setting the message string triggers an OOM, or
       capi.SQLITE_MISUSE if pDb is NULL or invalid (with the caveat
       that behavior in the later case is undefined if pDb is not
       "valid enough").

       Pass (pDb,0,0) to clear the error state.
    */
    util.sqlite3__wasm_db_error = function(pDb, resultCode, message){
      if( !pDb ) return capi.SQLITE_MISUSE;
      if(resultCode instanceof sqlite3.WasmAllocError){
        resultCode = capi.SQLITE_NOMEM;
        message = 0 /*avoid allocating message string*/;
      }else if(resultCode instanceof Error){
        message = message || ''+resultCode;
        resultCode = (resultCode.resultCode || capi.SQLITE_ERROR);
      }
      return capi.sqlite3_set_errmsg(pDb, resultCode, message) || resultCode;
    };
  }/*xWrap() bindings*/

  {/* Import C-level constants and structs... */
    const cJson = wasm.xCall('sqlite3__wasm_enum_json');
    if(!cJson){
      toss("Maintenance required: increase sqlite3__wasm_enum_json()'s",
           "static buffer size!");
    }
    wasm.ctype = JSON.parse(wasm.cstrToJs(cJson));
    // Groups of SQLITE_xyz macros...
    const defineGroups = ['access', 'authorizer',
                          'blobFinalizers', 'changeset',
                          'config', 'dataTypes',
                          'dbConfig', 'dbStatus',
                          'encodings', 'fcntl', 'flock', 'ioCap',
                          'limits', 'openFlags',
                          'prepareFlags', 'resultCodes',
                          'sqlite3Status',
                          'stmtStatus', 'syncFlags',
                          'trace', 'txnState', 'udfFlags',
                          'version'];
    if(wasm.bigIntEnabled){
      defineGroups.push('serialize', 'session', 'vtab');
    }
    for(const t of defineGroups){
      for(const e of Object.entries(wasm.ctype[t])){
        // ^^^ [k,v] there triggers a buggy code transformation via
        // one of the Emscripten-driven optimizers.
        capi[e[0]] = e[1];
      }
    }
    if(!wasm.functionEntry(capi.SQLITE_WASM_DEALLOC)){
      toss("Internal error: cannot resolve exported function",
           "entry SQLITE_WASM_DEALLOC (=="+capi.SQLITE_WASM_DEALLOC+").");
    }
    const __rcMap = Object.create(null);
    for(const e of Object.entries(wasm.ctype['resultCodes'])){
      __rcMap[e[1]] = e[0];
    }
    /**
       For the given integer, returns the SQLITE_xxx result code as a
       string, or undefined if no such mapping is found.
    */
    capi.sqlite3_js_rc_str = (rc)=>__rcMap[rc];

    /* Bind all registered C-side structs... */
    const notThese = Object.assign(Object.create(null),{
      // For each struct to NOT register, map its name to true:
      WasmTestStruct: true,
      /* sqlite3_index_info and friends require int64: */
      sqlite3_index_info: !wasm.bigIntEnabled,
      sqlite3_index_constraint: !wasm.bigIntEnabled,
      sqlite3_index_orderby: !wasm.bigIntEnabled,
      sqlite3_index_constraint_usage: !wasm.bigIntEnabled
    });
    for(const s of wasm.ctype.structs){
      if(!notThese[s.name]){
        capi[s.name] = sqlite3.StructBinder(s);
      }
    }
    if(capi.sqlite3_index_info){
      /* Move these inner structs into sqlite3_index_info.  Binding
      ** them to WASM requires that we create top-level structs to
      ** model them with, but those are no longer needed after we've
      ** passed them to StructBinder. */
      for(const k of ['sqlite3_index_constraint',
                      'sqlite3_index_orderby',
                      'sqlite3_index_constraint_usage']){
        capi.sqlite3_index_info[k] = capi[k];
        delete capi[k];
      }
      capi.sqlite3_vtab_config = wasm.xWrap(
        'sqlite3__wasm_vtab_config','int',[
          'sqlite3*', 'int', 'int']
      );
    }/* end vtab-related setup */
  }/*end C constant and struct imports*/

  /**
     Internal helper to assist in validating call argument counts in
     the hand-written sqlite3_xyz() wrappers. We do this only for
     consistency with non-special-case wrappings.
  */
  const __dbArgcMismatch = (pDb,f,n)=>{
    return util.sqlite3__wasm_db_error(pDb, capi.SQLITE_MISUSE,
                                      f+"() requires "+n+" argument"+
                                      (1===n?"":'s')+".");
  };

  /** Code duplication reducer for functions which take an encoding
      argument and require SQLITE_UTF8.  Sets the db error code to
      SQLITE_FORMAT, installs a descriptive error message,
      and returns SQLITE_FORMAT. */
  const __errEncoding = (pDb)=>{
    return util.sqlite3__wasm_db_error(
      pDb, capi.SQLITE_FORMAT, "SQLITE_UTF8 is the only supported encoding."
    );
  };

  /**
     __dbCleanupMap is infrastructure for recording registration of
     UDFs and collations so that sqlite3_close_v2() can clean up any
     automated JS-to-WASM function conversions installed by those.
  */
  const __argPDb = (pDb)=>wasm.xWrap.argAdapter('sqlite3*')(pDb);
  const __argStr = (str)=>wasm.isPtr(str) ? wasm.cstrToJs(str) : str;
  const __dbCleanupMap = function(
    pDb, mode/*0=remove, >0=create if needed, <0=do not create if missing*/
  ){
    pDb = __argPDb(pDb);
    let m = this.dbMap.get(pDb);
    if(!mode){
      this.dbMap.delete(pDb);
      return m;
    }else if(!m && mode>0){
      this.dbMap.set(pDb, (m = Object.create(null)));
    }
    return m;
  }.bind(Object.assign(Object.create(null),{
    dbMap: new Map
  }));

  __dbCleanupMap.addCollation = function(pDb, name){
    const m = __dbCleanupMap(pDb, 1);
    if(!m.collation) m.collation = new Set;
    m.collation.add(__argStr(name).toLowerCase());
  };

  __dbCleanupMap._addUDF = function(pDb, name, arity, map){
    /* Map UDF name to a Set of arity values */
    name = __argStr(name).toLowerCase();
    let u = map.get(name);
    if(!u) map.set(name, (u = new Set));
    u.add((arity<0) ? -1 : arity);
  };

  __dbCleanupMap.addFunction = function(pDb, name, arity){
    const m = __dbCleanupMap(pDb, 1);
    if(!m.udf) m.udf = new Map;
    this._addUDF(pDb, name, arity, m.udf);
  };

  if( wasm.exports.sqlite3_create_window_function ){
    __dbCleanupMap.addWindowFunc = function(pDb, name, arity){
      const m = __dbCleanupMap(pDb, 1);
      if(!m.wudf) m.wudf = new Map;
      this._addUDF(pDb, name, arity, m.wudf);
    };
  }

  /**
     Intended to be called _only_ from sqlite3_close_v2(),
     passed its non-0 db argument.

     This function frees up certain automatically-installed WASM
     function bindings which were installed on behalf of the given db,
     as those may otherwise leak.

     Notable caveat: this is only ever run via
     sqlite3.capi.sqlite3_close_v2(). If a client, for whatever
     reason, uses sqlite3.wasm.exports.sqlite3_close_v2() (the
     function directly exported from WASM), this cleanup will not
     happen.

     This is not a silver bullet for avoiding automation-related
     leaks but represents "an honest effort."

     The issue being addressed here is covered at:

     https://sqlite.org/wasm/doc/trunk/api-c-style.md#convert-func-ptr
  */
  __dbCleanupMap.cleanup = function(pDb){
    pDb = __argPDb(pDb);
    //wasm.xWrap.FuncPtrAdapter.debugFuncInstall = false;
    /**
       Installing NULL functions in the C API will remove those
       bindings. The FuncPtrAdapter which sits between us and the C
       API will also treat that as an opportunity to
       wasm.uninstallFunction() any WASM function bindings it has
       installed for pDb.
    */
    for(const obj of [
      /* pairs of [funcName, itsArityInC] */
      ['sqlite3_busy_handler',3],
      ['sqlite3_commit_hook',3],
      ['sqlite3_preupdate_hook',3],
      ['sqlite3_progress_handler',4],
      ['sqlite3_rollback_hook',3],
      ['sqlite3_set_authorizer',3],
      ['sqlite3_trace_v2', 4],
      ['sqlite3_update_hook',3]
      /*
        We do not yet have a way to clean up automatically-converted
        sqlite3_set_auxdata() finalizers.
      */
    ]){
      const [name, arity] = obj;
      const x = wasm.exports[name];
      if( !x ){
        /* assume it was built without this API */
        continue;
      }
      const closeArgs = [pDb];
      closeArgs.length = arity
      /* Recall that: (A) undefined entries translate to 0 when
         passed to WASM and (B) Safari wraps wasm.exports.* in
         nullary functions so x.length is 0 there. */;
      //wasm.xWrap.debug = true;
      try{ capi[name](...closeArgs) }
      catch(e){
        /* This "cannot happen" unless something is well and truly sideways. */
        sqlite3.config.warn("close-time call of",name+"(",closeArgs,") threw:",e);
      }
      //wasm.xWrap.debug = false;
    }
    const m = __dbCleanupMap(pDb, 0);
    if(!m) return;
    if(m.collation){
      for(const name of m.collation){
        try{
          capi.sqlite3_create_collation_v2(
            pDb, name, capi.SQLITE_UTF8, 0, 0, 0
          );
        }catch(e){
          /*ignored*/
        }
      }
      delete m.collation;
    }
    let i;
    for(i = 0; i < 2; ++i){ /* Clean up UDFs... */
      const fmap = i ? m.wudf : m.udf;
      if(!fmap) continue;
      const func = i
            ? capi.sqlite3_create_window_function
            : capi.sqlite3_create_function_v2;
      for(const e of fmap){
        const name = e[0], arities = e[1];
        const fargs = [pDb, name, 0/*arity*/, capi.SQLITE_UTF8, 0, 0, 0, 0, 0];
        if(i) fargs.push(0);
        for(const arity of arities){
          try{ fargs[2] = arity; func.apply(null, fargs); }
          catch(e){/*ignored*/}
        }
        arities.clear();
      }
      fmap.clear();
    }
    delete m.udf;
    delete m.wudf;
  }/*__dbCleanupMap.cleanup()*/;

  {/* Binding of sqlite3_close_v2() */
    const __sqlite3CloseV2 = wasm.xWrap("sqlite3_close_v2", "int", "sqlite3*");
    capi.sqlite3_close_v2 = function(pDb){
      if(1!==arguments.length) return __dbArgcMismatch(pDb, 'sqlite3_close_v2', 1);
      if(pDb){
        try{__dbCleanupMap.cleanup(pDb)} catch(e){/*ignored*/}
      }
      return __sqlite3CloseV2(pDb);
    };
  }/*sqlite3_close_v2()*/

  if(capi.sqlite3session_create){
    const __sqlite3SessionDelete = wasm.xWrap(
      'sqlite3session_delete', undefined, ['sqlite3_session*']
    );
    capi.sqlite3session_delete = function(pSession){
      if(1!==arguments.length){
        return __dbArgcMismatch(pDb, 'sqlite3session_delete', 1);
        /* Yes, we're returning a value from a void function. That seems
           like the lesser evil compared to not maintaining arg-count
           consistency as we do with other similar bindings. */
      }
      else if(pSession){
        //wasm.xWrap.FuncPtrAdapter.debugFuncInstall = true;
        capi.sqlite3session_table_filter(pSession, 0, 0);
      }
      __sqlite3SessionDelete(pSession);
    };
  }

  {/* Bindings for sqlite3_create_collation[_v2]() */
    // contextKey() impl for wasm.xWrap.FuncPtrAdapter
    const contextKey = (argv,argIndex)=>{
      return 'argv['+argIndex+']:'+argv[0/* sqlite3* */]+
        ':'+wasm.cstrToJs(argv[1/* collation name */]).toLowerCase()
    };
    const __sqlite3CreateCollationV2 = wasm.xWrap(
      'sqlite3_create_collation_v2', 'int', [
        'sqlite3*', 'string', 'int', '*',
        new wasm.xWrap.FuncPtrAdapter({
          /* int(*xCompare)(void*,int,const void*,int,const void*) */
          name: 'xCompare', signature: 'i(pipip)', contextKey
        }),
        new wasm.xWrap.FuncPtrAdapter({
          /* void(*xDestroy(void*) */
          name: 'xDestroy', signature: 'v(p)', contextKey
        })
      ]
    );

    /**
       Works exactly like C's sqlite3_create_collation_v2() except that:

       1) It returns capi.SQLITE_FORMAT if the 3rd argument contains
          any encoding-related value other than capi.SQLITE_UTF8.  No
          other encodings are supported. As a special case, if the
          bottom 4 bits of that argument are 0, SQLITE_UTF8 is
          assumed.

       2) It accepts JS functions for its function-pointer arguments,
          for which it will install WASM-bound proxies. The bindings
          are "permanent," in that they will stay in the WASM
          environment until it shuts down unless the client calls this
          again with the same collation name and a value of 0 or null
          for the the function pointer(s). sqlite3_close_v2() will
          also clean up such automatically-installed WASM functions.

       For consistency with the C API, it requires the same number of
       arguments. It returns capi.SQLITE_MISUSE if passed any other
       argument count.

       Returns 0 on success, non-0 on error, in which case the error
       state of pDb (of type `sqlite3*` or argument-convertible to it)
       may contain more information.
    */
    capi.sqlite3_create_collation_v2 = function(pDb,zName,eTextRep,pArg,xCompare,xDestroy){
      if(6!==arguments.length) return __dbArgcMismatch(pDb, 'sqlite3_create_collation_v2', 6);
      else if( 0 === (eTextRep & 0xf) ){
        eTextRep |= capi.SQLITE_UTF8;
      }else if( capi.SQLITE_UTF8 !== (eTextRep & 0xf) ){
        return __errEncoding(pDb);
      }
      try{
        const rc = __sqlite3CreateCollationV2(pDb, zName, eTextRep, pArg, xCompare, xDestroy);
        if(0===rc && xCompare instanceof Function){
          __dbCleanupMap.addCollation(pDb, zName);
        }
        return rc;
      }catch(e){
        return util.sqlite3__wasm_db_error(pDb, e);
      }
    };

    capi.sqlite3_create_collation = (pDb,zName,eTextRep,pArg,xCompare)=>{
      return (5===arguments.length)
        ? capi.sqlite3_create_collation_v2(pDb,zName,eTextRep,pArg,xCompare,0)
        : __dbArgcMismatch(pDb, 'sqlite3_create_collation', 5);
    };

  }/*sqlite3_create_collation() and friends*/

  {/* Special-case handling of sqlite3_create_function_v2()
      and sqlite3_create_window_function(). */
    /** FuncPtrAdapter for contextKey() for sqlite3_create_function()
        and friends. */
    const contextKey = function(argv,argIndex){
      return (
        argv[0/* sqlite3* */]
          +':'+(argv[2/*number of UDF args*/] < 0 ? -1 : argv[2])
          +':'+argIndex/*distinct for each xAbc callback type*/
          +':'+wasm.cstrToJs(argv[1]).toLowerCase()
      )
    };

    /**
       JS proxies for the various sqlite3_create[_window]_function()
       callbacks, structured in a form usable by wasm.xWrap.FuncPtrAdapter.
    */
    const __cfProxy = Object.assign(Object.create(null), {
      xInverseAndStep: {
        signature:'v(pip)', contextKey,
        callProxy: (callback)=>{
          return (pCtx, argc, pArgv)=>{
            try{ callback(pCtx, ...capi.sqlite3_values_to_js(argc, pArgv)) }
            catch(e){ capi.sqlite3_result_error_js(pCtx, e) }
          };
        }
      },
      xFinalAndValue: {
        signature:'v(p)', contextKey,
        callProxy: (callback)=>{
          return (pCtx)=>{
            try{ capi.sqlite3_result_js(pCtx, callback(pCtx)) }
            catch(e){ capi.sqlite3_result_error_js(pCtx, e) }
          };
        }
      },
      xFunc: {
        signature:'v(pip)', contextKey,
        callProxy: (callback)=>{
          return (pCtx, argc, pArgv)=>{
            try{
              capi.sqlite3_result_js(
                pCtx,
                callback(pCtx, ...capi.sqlite3_values_to_js(argc, pArgv))
              );
            }catch(e){
              //console.error('xFunc() caught:',e);
              capi.sqlite3_result_error_js(pCtx, e);
            }
          };
        }
      },
      xDestroy: {
        signature:'v(p)', contextKey,
        //Arguable: a well-behaved destructor doesn't require a proxy.
        callProxy: (callback)=>{
          return (pVoid)=>{
            try{ callback(pVoid) }
            catch(e){ console.error("UDF xDestroy method threw:",e) }
          };
        }
      }
    })/*__cfProxy*/;

    const __sqlite3CreateFunction = wasm.xWrap(
      "sqlite3_create_function_v2", "int", [
        "sqlite3*", "string"/*funcName*/, "int"/*nArg*/,
        "int"/*eTextRep*/, "*"/*pApp*/,
        new wasm.xWrap.FuncPtrAdapter({name: 'xFunc', ...__cfProxy.xFunc}),
        new wasm.xWrap.FuncPtrAdapter({name: 'xStep', ...__cfProxy.xInverseAndStep}),
        new wasm.xWrap.FuncPtrAdapter({name: 'xFinal', ...__cfProxy.xFinalAndValue}),
        new wasm.xWrap.FuncPtrAdapter({name: 'xDestroy', ...__cfProxy.xDestroy})
      ]
    );

    const __sqlite3CreateWindowFunction =
          wasm.exports.sqlite3_create_window_function
          ? wasm.xWrap(
            "sqlite3_create_window_function", "int", [
              "sqlite3*", "string"/*funcName*/, "int"/*nArg*/,
              "int"/*eTextRep*/, "*"/*pApp*/,
              new wasm.xWrap.FuncPtrAdapter({name: 'xStep', ...__cfProxy.xInverseAndStep}),
              new wasm.xWrap.FuncPtrAdapter({name: 'xFinal', ...__cfProxy.xFinalAndValue}),
              new wasm.xWrap.FuncPtrAdapter({name: 'xValue', ...__cfProxy.xFinalAndValue}),
              new wasm.xWrap.FuncPtrAdapter({name: 'xInverse', ...__cfProxy.xInverseAndStep}),
              new wasm.xWrap.FuncPtrAdapter({name: 'xDestroy', ...__cfProxy.xDestroy})
            ]
          )
          : undefined;

    /* Documented in the api object's initializer. */
    capi.sqlite3_create_function_v2 = function f(
      pDb, funcName, nArg, eTextRep, pApp,
      xFunc,   //void (*xFunc)(sqlite3_context*,int,sqlite3_value**)
      xStep,   //void (*xStep)(sqlite3_context*,int,sqlite3_value**)
      xFinal,  //void (*xFinal)(sqlite3_context*)
      xDestroy //void (*xDestroy)(void*)
    ){
      if( f.length!==arguments.length ){
        return __dbArgcMismatch(pDb,"sqlite3_create_function_v2",f.length);
      }else if( 0 === (eTextRep & 0xf) ){
        eTextRep |= capi.SQLITE_UTF8;
      }else if( capi.SQLITE_UTF8 !== (eTextRep & 0xf) ){
        return __errEncoding(pDb);
      }
      try{
        const rc = __sqlite3CreateFunction(pDb, funcName, nArg, eTextRep,
                                           pApp, xFunc, xStep, xFinal, xDestroy);
        if(0===rc && (xFunc instanceof Function
                      || xStep instanceof Function
                      || xFinal instanceof Function
                      || xDestroy instanceof Function)){
          __dbCleanupMap.addFunction(pDb, funcName, nArg);
        }
        return rc;
      }catch(e){
        console.error("sqlite3_create_function_v2() setup threw:",e);
        return util.sqlite3__wasm_db_error(pDb, e, "Creation of UDF threw: "+e);
      }
    };

    /* Documented in the api object's initializer. */
    capi.sqlite3_create_function = function f(
      pDb, funcName, nArg, eTextRep, pApp,
      xFunc, xStep, xFinal
    ){
      return (f.length===arguments.length)
        ? capi.sqlite3_create_function_v2(pDb, funcName, nArg, eTextRep,
                                          pApp, xFunc, xStep, xFinal, 0)
        : __dbArgcMismatch(pDb,"sqlite3_create_function",f.length);
    };

    /* Documented in the api object's initializer. */
    if( __sqlite3CreateWindowFunction ){
      capi.sqlite3_create_window_function = function f(
        pDb, funcName, nArg, eTextRep, pApp,
        xStep,   //void (*xStep)(sqlite3_context*,int,sqlite3_value**)
        xFinal,  //void (*xFinal)(sqlite3_context*)
        xValue,  //void (*xValue)(sqlite3_context*)
        xInverse,//void (*xInverse)(sqlite3_context*,int,sqlite3_value**)
        xDestroy //void (*xDestroy)(void*)
      ){
        if( f.length!==arguments.length ){
          return __dbArgcMismatch(pDb,"sqlite3_create_window_function",f.length);
        }else if( 0 === (eTextRep & 0xf) ){
          eTextRep |= capi.SQLITE_UTF8;
        }else if( capi.SQLITE_UTF8 !== (eTextRep & 0xf) ){
          return __errEncoding(pDb);
        }
        try{
          const rc = __sqlite3CreateWindowFunction(pDb, funcName, nArg, eTextRep,
                                                   pApp, xStep, xFinal, xValue,
                                                   xInverse, xDestroy);
          if(0===rc && (xStep instanceof Function
                        || xFinal instanceof Function
                        || xValue instanceof Function
                        || xInverse instanceof Function
                        || xDestroy instanceof Function)){
            __dbCleanupMap.addWindowFunc(pDb, funcName, nArg);
          }
          return rc;
        }catch(e){
          console.error("sqlite3_create_window_function() setup threw:",e);
          return util.sqlite3__wasm_db_error(pDb, e, "Creation of UDF threw: "+e);
        }
      };
    }else{
      delete capi.sqlite3_create_window_function;
    }
    /**
       A _deprecated_ alias for capi.sqlite3_result_js() which
       predates the addition of that function in the public API.
    */
    capi.sqlite3_create_function_v2.udfSetResult =
      capi.sqlite3_create_function.udfSetResult = capi.sqlite3_result_js;
    if(capi.sqlite3_create_window_function){
      capi.sqlite3_create_window_function.udfSetResult = capi.sqlite3_result_js;
    }

    /**
       A _deprecated_ alias for capi.sqlite3_values_to_js() which
       predates the addition of that function in the public API.
    */
    capi.sqlite3_create_function_v2.udfConvertArgs =
      capi.sqlite3_create_function.udfConvertArgs = capi.sqlite3_values_to_js;
    if(capi.sqlite3_create_window_function){
      capi.sqlite3_create_window_function.udfConvertArgs = capi.sqlite3_values_to_js;
    }

    /**
       A _deprecated_ alias for capi.sqlite3_result_error_js() which
       predates the addition of that function in the public API.
    */
    capi.sqlite3_create_function_v2.udfSetError =
      capi.sqlite3_create_function.udfSetError = capi.sqlite3_result_error_js;
    if(capi.sqlite3_create_window_function){
      capi.sqlite3_create_window_function.udfSetError = capi.sqlite3_result_error_js;
    }

  }/*sqlite3_create_function_v2() and sqlite3_create_window_function() proxies*/;

  {/* Special-case handling of sqlite3_prepare_v2() and
      sqlite3_prepare_v3() */

    /**
       Helper for string:flexible conversions which requires a
       byte-length counterpart argument. Passed a value and its
       ostensible length, this function returns [V,N], where V is
       either v or a to-string transformed copy of v and N is either n
       (if v is a WASM pointer, in which case n might be a BigInt), -1
       (if v is a string or Array), or the byte length of v (if it's a
       byte array or ArrayBuffer).
    */
    const __flexiString = (v,n)=>{
      if('string'===typeof v){
        n = -1;
      }else if(util.isSQLableTypedArray(v)){
        n = v.byteLength;
        v = wasm.typedArrayToString(
          (v instanceof ArrayBuffer) ? new Uint8Array(v) : v
        );
      }else if(Array.isArray(v)){
        v = v.join("");
        n = -1;
      }
      return [v, n];
    };

    /**
       Scope-local holder of the two impls of sqlite3_prepare_v2/v3().
    */
    const __prepare = {
      /**
         This binding expects a JS string as its 2nd argument and
         null as its final argument. In order to compile multiple
         statements from a single string, the "full" impl (see
         below) must be used.
      */
      basic: wasm.xWrap('sqlite3_prepare_v3',
                        "int", ["sqlite3*", "string",
                                "int"/*ignored for this impl!*/,
                                "int", "**",
                                "**"/*MUST be 0 or null or undefined!*/]),
      /**
         Impl which requires that the 2nd argument be a pointer to the
         SQL string, instead of being converted to a JS string. This
         variant is necessary for cases where we require a non-NULL
         value for the final argument (prepare/step of multiple
         statements from one input string). For simpler cases, where
         only the first statement in the SQL string is required, the
         wrapper named sqlite3_prepare_v2() is sufficient and easier
         to use because it doesn't require dealing with pointers.
      */
      full: wasm.xWrap('sqlite3_prepare_v3',
                       "int", ["sqlite3*", "*", "int", "int",
                               "**", "**"])
    };

    /* Documented in the capi object's initializer. */
    capi.sqlite3_prepare_v3 = function f(pDb, sql, sqlLen, prepFlags, ppStmt, pzTail){
      if(f.length!==arguments.length){
        return __dbArgcMismatch(pDb,"sqlite3_prepare_v3",f.length);
      }
      const [xSql, xSqlLen] = __flexiString(sql, Number(sqlLen));
      switch(typeof xSql){
        case 'string': return __prepare.basic(pDb, xSql, xSqlLen, prepFlags, ppStmt, null);
        case (typeof wasm.ptr.null):
          return __prepare.full(pDb, wasm.ptr.coerce(xSql), xSqlLen, prepFlags,
                                ppStmt, pzTail);
        default:
          return util.sqlite3__wasm_db_error(
            pDb, capi.SQLITE_MISUSE,
            "Invalid SQL argument type for sqlite3_prepare_v2/v3(). typeof="+(typeof xSql)
          );
      }
    };

    /* Documented in the capi object's initializer. */
    capi.sqlite3_prepare_v2 = function f(pDb, sql, sqlLen, ppStmt, pzTail){
      return (f.length===arguments.length)
        ? capi.sqlite3_prepare_v3(pDb, sql, sqlLen, 0, ppStmt, pzTail)
        : __dbArgcMismatch(pDb,"sqlite3_prepare_v2",f.length);
    };

  }/*sqlite3_prepare_v2/v3()*/

  {/*sqlite3_bind_text/blob()*/
    const __bindText = wasm.xWrap("sqlite3_bind_text", "int", [
      "sqlite3_stmt*", "int", "string", "int", "*"
    ]);
    const __bindBlob = wasm.xWrap("sqlite3_bind_blob", "int", [
      "sqlite3_stmt*", "int", "*", "int", "*"
    ]);

    /** Documented in the capi object's initializer. */
    capi.sqlite3_bind_text = function f(pStmt, iCol, text, nText, xDestroy){
      if(f.length!==arguments.length){
        return __dbArgcMismatch(capi.sqlite3_db_handle(pStmt),
                                "sqlite3_bind_text", f.length);
      }else if(wasm.isPtr(text) || null===text){
        return __bindText(pStmt, iCol, text, nText, xDestroy);
      }else if(text instanceof ArrayBuffer){
        text = new Uint8Array(text);
      }else if(Array.isArray(pMem)){
        text = pMem.join('');
      }
      let p, n;
      try{
        if(util.isSQLableTypedArray(text)){
          p = wasm.allocFromTypedArray(text);
          n = text.byteLength;
        }else if('string'===typeof text){
          [p, n] = wasm.allocCString(text);
        }else{
          return util.sqlite3__wasm_db_error(
            capi.sqlite3_db_handle(pStmt), capi.SQLITE_MISUSE,
            "Invalid 3rd argument type for sqlite3_bind_text()."
          );
        }
        return __bindText(pStmt, iCol, p, n, capi.SQLITE_WASM_DEALLOC);
      }catch(e){
        wasm.dealloc(p);
        return util.sqlite3__wasm_db_error(
          capi.sqlite3_db_handle(pStmt), e
        );
      }
    }/*sqlite3_bind_text()*/;

    /** Documented in the capi object's initializer. */
    capi.sqlite3_bind_blob = function f(pStmt, iCol, pMem, nMem, xDestroy){
      if(f.length!==arguments.length){
        return __dbArgcMismatch(capi.sqlite3_db_handle(pStmt),
                                "sqlite3_bind_blob", f.length);
      }else if(wasm.isPtr(pMem) || null===pMem){
        return __bindBlob(pStmt, iCol, pMem, nMem, xDestroy);
      }else if(pMem instanceof ArrayBuffer){
        pMem = new Uint8Array(pMem);
      }else if(Array.isArray(pMem)){
        pMem = pMem.join('');
      }
      let p, n;
      try{
        if(util.isBindableTypedArray(pMem)){
          p = wasm.allocFromTypedArray(pMem);
          n = nMem>=0 ? nMem : pMem.byteLength;
        }else if('string'===typeof pMem){
          [p, n] = wasm.allocCString(pMem);
        }else{
          return util.sqlite3__wasm_db_error(
            capi.sqlite3_db_handle(pStmt), capi.SQLITE_MISUSE,
            "Invalid 3rd argument type for sqlite3_bind_blob()."
          );
        }
        return __bindBlob(pStmt, iCol, p, n, capi.SQLITE_WASM_DEALLOC);
      }catch(e){
        wasm.dealloc(p);
        return util.sqlite3__wasm_db_error(
          capi.sqlite3_db_handle(pStmt), e
        );
      }
    }/*sqlite3_bind_blob()*/;

  }/*sqlite3_bind_text/blob()*/

  if(!capi.sqlite3_column_text){
    /*[tag:proxy-text-apis]
      As discussed at:

      https://sqlite.org/forum/forumpost/d77281aec2df9ada

      Summary: there are opinions that sqlite3_column_text() and
      sqlite3_value_text() should handle strings such that embedded
      NULs are retained. This block does that. This block does _not_
      apply that special-case behavior to any number of _other_
      APIs which return C-strings. That discrepancy makes this
      block highly arguable, but one can also argue that these two
      specific functions can get away with such acrobatics without
      it being called voodoo in a pejorative sense.
    */
    const argStmt  = wasm.xWrap.argAdapter('sqlite3_stmt*'),
          argInt   = wasm.xWrap.argAdapter('int'),
          argValue = wasm.xWrap.argAdapter('sqlite3_value*'),
          newStr   =
          (cstr,n)=>wasm.typedArrayToString(wasm.heap8u(),
                                           Number(cstr), Number(cstr)+n)
    capi.sqlite3_column_text = function(stmt, colIndex){
      const a0 = argStmt(stmt), a1 = argInt(colIndex);
      const cstr = wasm.exports.sqlite3_column_text(a0, a1);
      return cstr
        ? newStr(cstr,wasm.exports.sqlite3_column_bytes(a0, a1))
        : null;
    };
    capi.sqlite3_value_text = function(val){
      const a0 = argValue(val);
      const cstr = wasm.exports.sqlite3_value_text(a0);
      return cstr
        ? newStr(cstr,wasm.exports.sqlite3_value_bytes(a0))
        : null;
    };
  }/*text-return-related bindings*/

  {/* sqlite3_config() */
    /**
       Wraps a small subset of the C API's sqlite3_config() options.
       Unsupported options trigger the return of capi.SQLITE_NOTFOUND.
       Passing fewer than 2 arguments triggers return of
       capi.SQLITE_MISUSE.
    */
    capi.sqlite3_config = function(op, ...args){
      if(arguments.length<2) return capi.SQLITE_MISUSE;
      switch(op){
          case capi.SQLITE_CONFIG_COVERING_INDEX_SCAN: // 20  /* int */
          case capi.SQLITE_CONFIG_MEMSTATUS:// 9  /* boolean */
          case capi.SQLITE_CONFIG_SMALL_MALLOC: // 27  /* boolean */
          case capi.SQLITE_CONFIG_SORTERREF_SIZE: // 28  /* int nByte */
          case capi.SQLITE_CONFIG_STMTJRNL_SPILL: // 26  /* int nByte */
          case capi.SQLITE_CONFIG_URI:// 17  /* int */
            return wasm.exports.sqlite3__wasm_config_i(op, args[0]);
          case capi.SQLITE_CONFIG_LOOKASIDE: // 13  /* int int */
            return wasm.exports.sqlite3__wasm_config_ii(op, args[0], args[1]);
          case capi.SQLITE_CONFIG_MEMDB_MAXSIZE: // 29  /* sqlite3_int64 */
            return wasm.exports.sqlite3__wasm_config_j(op, args[0]);
          case capi.SQLITE_CONFIG_GETMALLOC: // 5 /* sqlite3_mem_methods* */
          case capi.SQLITE_CONFIG_GETMUTEX: // 11  /* sqlite3_mutex_methods* */
          case capi.SQLITE_CONFIG_GETPCACHE2: // 19  /* sqlite3_pcache_methods2* */
          case capi.SQLITE_CONFIG_GETPCACHE: // 15  /* no-op */
          case capi.SQLITE_CONFIG_HEAP: // 8  /* void*, int nByte, int min */
          case capi.SQLITE_CONFIG_LOG: // 16  /* xFunc, void* */
          case capi.SQLITE_CONFIG_MALLOC:// 4  /* sqlite3_mem_methods* */
          case capi.SQLITE_CONFIG_MMAP_SIZE: // 22  /* sqlite3_int64, sqlite3_int64 */
          case capi.SQLITE_CONFIG_MULTITHREAD: // 2 /* nil */
          case capi.SQLITE_CONFIG_MUTEX: // 10  /* sqlite3_mutex_methods* */
          case capi.SQLITE_CONFIG_PAGECACHE: // 7  /* void*, int sz, int N */
          case capi.SQLITE_CONFIG_PCACHE2: // 18  /* sqlite3_pcache_methods2* */
          case capi.SQLITE_CONFIG_PCACHE: // 14  /* no-op */
          case capi.SQLITE_CONFIG_PCACHE_HDRSZ: // 24  /* int *psz */
          case capi.SQLITE_CONFIG_PMASZ: // 25  /* unsigned int szPma */
          case capi.SQLITE_CONFIG_SERIALIZED: // 3 /* nil */
          case capi.SQLITE_CONFIG_SINGLETHREAD: // 1 /* nil */:
          case capi.SQLITE_CONFIG_SQLLOG: // 21  /* xSqllog, void* */
          case capi.SQLITE_CONFIG_WIN32_HEAPSIZE: // 23  /* int nByte */
          default:
          /* maintenance note: we specifically do not include
             SQLITE_CONFIG_ROWID_IN_VIEW here, on the grounds that
             it's only for legacy support and no apps written with
             this API require that. */
            return capi.SQLITE_NOTFOUND;
      }
    };
  }/* sqlite3_config() */

  {/*auto-extension bindings.*/
    const __autoExtFptr = new Set;

    capi.sqlite3_auto_extension = function(fPtr){
      if( fPtr instanceof Function ){
        fPtr = wasm.installFunction('i(ppp)', fPtr);
      }else if( 1!==arguments.length || !wasm.isPtr(fPtr) ){
        return capi.SQLITE_MISUSE;
      }
      const rc = wasm.exports.sqlite3_auto_extension(fPtr);
      if( fPtr!==arguments[0] ){
        if(0===rc) __autoExtFptr.add(fPtr);
        else wasm.uninstallFunction(fPtr);
      }
      return rc;
    };

    capi.sqlite3_cancel_auto_extension = function(fPtr){
     /* We do not do an automatic JS-to-WASM function conversion here
        because it would be senseless: the converted pointer would
        never possibly match an already-installed one. */;
      if(!fPtr || 1!==arguments.length || !wasm.isPtr(fPtr)) return 0;
      return wasm.exports.sqlite3_cancel_auto_extension(fPtr);
      /* Note that it "cannot happen" that a client passes a pointer which
         is in __autoExtFptr because __autoExtFptr only contains automatic
         conversions created inside sqlite3_auto_extension() and
         never exposed to the client. */
    };

    capi.sqlite3_reset_auto_extension = function(){
      wasm.exports.sqlite3_reset_auto_extension();
      for(const fp of __autoExtFptr) wasm.uninstallFunction(fp);
      __autoExtFptr.clear();
    };
  }/* auto-extension */

  /* Warn if client-level code makes use of FuncPtrAdapter. */
  wasm.xWrap.FuncPtrAdapter.warnOnUse = true;

  const StructBinder = sqlite3.StructBinder
  /* we require a local alias b/c StructBinder is removed from the sqlite3
     object during the final steps of the API cleanup. */;
  /**
     Installs a StructBinder-bound function pointer member of the
     given name and function in the given StructBinder.StructType
     target object.

     It creates a WASM proxy for the given function and arranges for
     that proxy to be cleaned up when tgt.dispose() is called. Throws
     on the slightest hint of error, e.g. tgt is-not-a StructType,
     name does not map to a struct-bound member, etc.

     As a special case, if the given function is a pointer, then
     `wasm.functionEntry()` is used to validate that it is a known
     function. If so, it is used as-is with no extra level of proxying
     or cleanup, else an exception is thrown. It is legal to pass a
     value of 0, indicating a NULL pointer, with the caveat that 0
     _is_ a legal function pointer in WASM but it will not be accepted
     as such _here_. (Justification: the function at address zero must
     be one which initially came from the WASM module, not a method we
     want to bind to a virtual table or VFS.)

     This function returns a proxy for itself which is bound to tgt
     and takes 2 args (name,func). That function returns the same
     thing as this one, permitting calls to be chained.

     If called with only 1 arg, it has no side effects but returns a
     func with the same signature as described above.

     ACHTUNG: because we cannot generically know how to transform JS
     exceptions into result codes, the installed functions do no
     automatic catching of exceptions. It is critical, to avoid
     undefined behavior in the C layer, that methods mapped via
     this function do not throw. The exception, as it were, to that
     rule is...

     If applyArgcCheck is true then each JS function (as opposed to
     function pointers) gets wrapped in a proxy which asserts that it
     is passed the expected number of arguments, throwing if the
     argument count does not match expectations. That is only intended
     for dev-time usage for sanity checking, and may leave the C
     environment in an undefined state.
  */
  const installMethod = function callee(
    tgt, name, func, applyArgcCheck = callee.installMethodArgcCheck
  ){
    if(!(tgt instanceof StructBinder.StructType)){
      toss("Usage error: target object is-not-a StructType.");
    }else if(!(func instanceof Function) && !wasm.isPtr(func)){
      toss("Usage error: expecting a Function or WASM pointer to one.");
    }
    if(1===arguments.length){
      return (n,f)=>callee(tgt, n, f, applyArgcCheck);
    }
    if(!callee.argcProxy){
      callee.argcProxy = function(tgt, funcName, func,sig){
        return function(...args){
          if(func.length!==arguments.length){
            toss("Argument mismatch for",
                 tgt.structInfo.name+"::"+funcName
                 +": Native signature is:",sig);
          }
          return func.apply(this, args);
        }
      };
      /* An ondispose() callback for use with
         StructBinder-created types. */
      callee.removeFuncList = function(){
        if(this.ondispose.__removeFuncList){
          this.ondispose.__removeFuncList.forEach(
            (v,ndx)=>{
              if(wasm.isPtr(v)){
                try{wasm.uninstallFunction(v)}
                catch(e){/*ignore*/}
              }
              /* else it's a descriptive label for the next number in
                 the list. */
            }
          );
          delete this.ondispose.__removeFuncList;
        }
      };
    }/*static init*/
    const sigN = tgt.memberSignature(name);
    if(sigN.length<2){
      toss("Member",name,"does not have a function pointer signature:",sigN);
    }
    const memKey = tgt.memberKey(name);
    const fProxy = (applyArgcCheck && !wasm.isPtr(func))
    /** This middle-man proxy is only for use during development, to
        confirm that we always pass the proper number of
        arguments. We know that the C-level code will always use the
        correct argument count. */
          ? callee.argcProxy(tgt, memKey, func, sigN)
          : func;
    if(wasm.isPtr(fProxy)){
      if(fProxy && !wasm.functionEntry(fProxy)){
        toss("Pointer",fProxy,"is not a WASM function table entry.");
      }
      tgt[memKey] = fProxy;
    }else{
      const pFunc = wasm.installFunction(fProxy, sigN);
      tgt[memKey] = pFunc;
      if(!tgt.ondispose || !tgt.ondispose.__removeFuncList){
        tgt.addOnDispose('ondispose.__removeFuncList handler',
                         callee.removeFuncList);
        tgt.ondispose.__removeFuncList = [];
      }
      tgt.ondispose.__removeFuncList.push(memKey, pFunc);
    }
    return (n,f)=>callee(tgt, n, f, applyArgcCheck);
  }/*installMethod*/;
  installMethod.installMethodArgcCheck = false;

  /**
     Installs methods into the given StructBinder.StructType-type
     instance. Each entry in the given methods object must map to a
     known member of the given StructType, else an exception will be
     triggered.  See installMethod() for more details, including the
     semantics of the 3rd argument.

     As an exception to the above, if any two or more methods in the
     2nd argument are the exact same function, installMethod() is
     _not_ called for the 2nd and subsequent instances, and instead
     those instances get assigned the same method pointer which is
     created for the first instance. This optimization is primarily to
     accommodate special handling of sqlite3_module::xConnect and
     xCreate methods.

     On success, returns its first argument. Throws on error.
  */
  const installMethods = function(
    structInstance, methods, applyArgcCheck = installMethod.installMethodArgcCheck
  ){
    const seen = new Map /* map of <Function, memberName> */;
    for(const k of Object.keys(methods)){
      const m = methods[k];
      const prior = seen.get(m);
      if(prior){
        const mkey = structInstance.memberKey(k);
        structInstance[mkey] = structInstance[structInstance.memberKey(prior)];
      }else{
        installMethod(structInstance, k, m, applyArgcCheck);
        seen.set(m, k);
      }
    }
    return structInstance;
  };

  /**
     Equivalent to calling installMethod(this,...arguments) with a
     first argument of this object. If called with 1 or 2 arguments
     and the first is an object, it's instead equivalent to calling
     installMethods(this,...arguments).
  */
  StructBinder.StructType.prototype.installMethod = function callee(
    name, func, applyArgcCheck = installMethod.installMethodArgcCheck
  ){
    return (arguments.length < 3 && name && 'object'===typeof name)
      ? installMethods(this, ...arguments)
      : installMethod(this, ...arguments);
  };

  /**
     Equivalent to calling installMethods() with a first argument
     of this object.
  */
  StructBinder.StructType.prototype.installMethods = function(
    methods, applyArgcCheck = installMethod.installMethodArgcCheck
  ){
    return installMethods(this, methods, applyArgcCheck);
  };

});
/*
  2022-07-22

  The author disclaims copyright to this source code.  In place of a
  legal notice, here is a blessing:

  *   May you do good and not evil.
  *   May you find forgiveness for yourself and forgive others.
  *   May you share freely, never taking more than you give.

  ***********************************************************************

  This file contains the so-called OO #1 API wrapper for the sqlite3
  WASM build. It requires that sqlite3-api-glue.js has already run
  and it installs its deliverable as sqlite3.oo1.
*/
globalThis.sqlite3ApiBootstrap.initializers.push(function(sqlite3){
  const toss3 = (...args)=>{throw new sqlite3.SQLite3Error(...args)};

  const capi = sqlite3.capi, wasm = sqlite3.wasm, util = sqlite3.util;
  /* What follows is colloquially known as "OO API #1". It is a
     binding of the sqlite3 API which is designed to be run within
     the same thread (main or worker) as the one in which the
     sqlite3 WASM binding was initialized. This wrapper cannot use
     the sqlite3 binding if, e.g., the wrapper is in the main thread
     and the sqlite3 API is in a worker. */

  const outWrapper = function(f){
    return (...args)=>f("sqlite3.oo1:",...args);
  };

  const debug = sqlite3.__isUnderTest
        ? outWrapper(console.debug.bind(console))
        : outWrapper(sqlite3.config.debug);
  const warn = sqlite3.__isUnderTest
        ? outWrapper(console.warn.bind(console))
        : outWrapper(sqlite3.config.warn);
  const error = sqlite3.__isUnderTest
        ? outWrapper(console.error.bind(console))
        : outWrapper(sqlite3.config.error);

  /**
     In order to keep clients from manipulating, perhaps
     inadvertently, the underlying pointer values of DB and Stmt
     instances, we'll gate access to them via the `pointer` property
     accessor and store their real values in this map. Keys = DB/Stmt
     objects, values = pointer values. This also unifies how those are
     accessed, for potential use downstream via custom
     wasm.xWrap() function signatures which know how to extract
     it.
  */
  const __ptrMap = new WeakMap();
  /**
     A Set of oo1.DB or oo1.Stmt objects which are proxies for
     (sqlite3*) resp. (sqlite3_stmt*) pointers which themselves are
     owned elsewhere. Objects in this Set do not own their underlying
     handle and that handle must be guaranteed (by the client) to
     outlive the proxy. DB.close()/Stmt.finalize() methods will remove
     the object from this Set _instead_ of closing/finalizing the
     pointer. These proxies are primarily intended as a way to briefly
     wrap an (sqlite3[_stmt]*) object as an oo1.DB/Stmt without taking
     over ownership, to take advantage of simplifies usage compared to
     the C API while not imposing any change of ownership.

     See DB.wrapHandle() and Stmt.wrapHandle().
  */
  const __doesNotOwnHandle = new Set();
  /**
     Map of DB instances to objects, each object being a map of Stmt
     wasm pointers to Stmt objects.
  */
  const __stmtMap = new WeakMap();

  /** If object opts has _its own_ property named p then that
      property's value is returned, else dflt is returned. */
  const getOwnOption = (opts, p, dflt)=>{
    const d = Object.getOwnPropertyDescriptor(opts,p);
    return d ? d.value : dflt;
  };

  // Documented in DB.checkRc()
  const checkSqlite3Rc = function(dbPtr, sqliteResultCode){
    if(sqliteResultCode){
      if(dbPtr instanceof DB) dbPtr = dbPtr.pointer;
      toss3(
        sqliteResultCode,
        "sqlite3 result code",sqliteResultCode+":",
        (dbPtr
         ? capi.sqlite3_errmsg(dbPtr)
         : capi.sqlite3_errstr(sqliteResultCode))
      );
    }
    return arguments[0];
  };

  /**
     sqlite3_trace_v2() callback which gets installed by the DB ctor
     if its open-flags contain "t".
  */
  const __dbTraceToConsole =
        wasm.installFunction('i(ippp)', function(t,c,p,x){
          if(capi.SQLITE_TRACE_STMT===t){
            // x == SQL, p == sqlite3_stmt*
            console.log("SQL TRACE #"+(++this.counter),
                        'via sqlite3@'+c+'['+capi.sqlite3_db_filename(c,null)+']',
                        wasm.cstrToJs(x));
          }
        }.bind({counter: 0}));

  /**
     A map of sqlite3_vfs pointers to SQL code or a callback function
     to run when the DB constructor opens a database with the given
     VFS. In the latter case, the call signature is
     (theDbObject,sqlite3Namespace) and the callback is expected to
     throw on error.
  */
  const __vfsPostOpenCallback = Object.create(null);


  /**
     A proxy for DB class constructors. It must be called with the
     being-construct DB object as its "this". See the DB constructor
     for the argument docs. This is split into a separate function
     in order to enable simple creation of special-case DB constructors,
     e.g. JsStorageDb and OpfsDb.

     Expects to be passed a configuration object with the following
     properties:

     - `.filename`: the db filename. It may be a special name like ":memory:"
       or "". It may also be a URI-style name.

     - `.flags`: as documented in the DB constructor.

     - `.vfs`: as documented in the DB constructor.

     It also accepts those as the first 3 arguments.

     In non-default builds it may accept additional configuration
     options.
  */
  const dbCtorHelper = function ctor(...args){
    const opt = ctor.normalizeArgs(...args);
    //sqlite3.config.debug("DB ctor",opt);
    let pDb;
    if( (pDb = opt['sqlite3*']) ){
      /* This property ^^^^^ is very specifically NOT DOCUMENTED and
         NOT part of the public API. This is a back door for functions
         like DB.wrapDbHandle(). */
      //sqlite3.config.debug("creating proxy db from",opt);
      if( !opt['sqlite3*:takeOwnership'] ){
        /* This is object does not own its handle. */
        __doesNotOwnHandle.add(this);
      }
      this.filename = capi.sqlite3_db_filename(pDb,'main');
    }else{
      let fn = opt.filename, vfsName = opt.vfs, flagsStr = opt.flags;
      if( ('string'!==typeof fn && !wasm.isPtr(fn))
          || 'string'!==typeof flagsStr
          || (vfsName && ('string'!==typeof vfsName && !wasm.isPtr(vfsName))) ){
        sqlite3.config.error("Invalid DB ctor args",opt,arguments);
        toss3("Invalid arguments for DB constructor:", arguments, "opts:", opt);
      }
      let oflags = 0;
      if( flagsStr.indexOf('c')>=0 ){
        oflags |= capi.SQLITE_OPEN_CREATE | capi.SQLITE_OPEN_READWRITE;
      }
      if( flagsStr.indexOf('w')>=0 ) oflags |= capi.SQLITE_OPEN_READWRITE;
      if( 0===oflags ) oflags |= capi.SQLITE_OPEN_READONLY;
      oflags |= capi.SQLITE_OPEN_EXRESCODE;
      const stack = wasm.pstack.pointer;
      try {
        const pPtr = wasm.pstack.allocPtr() /* output (sqlite3**) arg */;
        let rc = capi.sqlite3_open_v2(fn, pPtr, oflags, vfsName || wasm.ptr.null);
        pDb = wasm.peekPtr(pPtr);
        checkSqlite3Rc(pDb, rc);
        capi.sqlite3_extended_result_codes(pDb, 1);
        if(flagsStr.indexOf('t')>=0){
          capi.sqlite3_trace_v2(pDb, capi.SQLITE_TRACE_STMT,
                                __dbTraceToConsole, pDb);
        }
      }catch( e ){
        if( pDb ) capi.sqlite3_close_v2(pDb);
        throw e;
      }finally{
        wasm.pstack.restore(stack);
      }
      this.filename =
        /* A poor design choice we have to keep: this.filename may be
           in the form "file:....?....". It really should have been
           sqlite3_db_filename(pDb) but that discrepancy went too long
           unnoticed to be able to change without risk of
           breakage. DB.dbFilename() can be used to fetch _just_ the
           name part.
        */ wasm.isPtr(fn) ? wasm.cstrToJs(fn) : fn;

    }
    __ptrMap.set(this, pDb);
    __stmtMap.set(this, Object.create(null));
    if( !opt['sqlite3*'] ){
      try{
        // Check for per-VFS post-open SQL/callback...
        const pVfs = capi.sqlite3_js_db_vfs(pDb)
              || toss3("Internal error: cannot get VFS for new db handle.");
        const postInitSql = __vfsPostOpenCallback[pVfs];
        if(postInitSql){
          /**
             Reminder: if this db is encrypted and the client did _not_ pass
             in the key, any init code will fail, causing the ctor to throw.
             We don't actually know whether the db is encrypted, so we cannot
             sensibly apply any heuristics which skip the init code only for
             encrypted databases for which no key has yet been supplied.
          */
          if(postInitSql instanceof Function){
            postInitSql(this, sqlite3);
          }else{
            checkSqlite3Rc(
              pDb, capi.sqlite3_exec(pDb, postInitSql, 0, 0, 0)
            );
          }
        }
      }catch(e){
        this.close();
        throw e;
      }
    }
  };

  /**
     Sets a callback which should be called after a db is opened with
     the given sqlite3_vfs pointer. The 2nd argument must be a
     function, which gets called with
     (theOo1DbObject,sqlite3Namespace) at the end of the DB()
     constructor. The function must throw on error, in which case the
     db is closed and the exception is propagated.  This function is
     intended only for use by DB subclasses or sqlite3_vfs
     implementations.

     Prior to 2024-07-22, it was legal to pass SQL code as the second
     argument, but that can interfere with a client's ability to run
     pragmas which must be run before anything else, namely (pragma
     locking_mode=exclusive) for use with WAL mode.  That capability
     had only ever been used as an internal detail of the two OPFS
     VFSes, and they no longer use it that way.
  */
  dbCtorHelper.setVfsPostOpenCallback = function(pVfs, callback){
    if( !(callback instanceof Function)){
      toss3("dbCtorHelper.setVfsPostOpenCallback() should not be used with "+
            "a non-function argument.",arguments);
    }
    __vfsPostOpenCallback[pVfs] = callback;
  };

  /**
     A helper for DB constructors. It accepts either a single
     config-style object or up to 3 arguments (filename, dbOpenFlags,
     dbVfsName). It returns a new object containing:

     { filename: ..., flags: ..., vfs: ... }

     If passed an object, any additional properties it has are copied
     as-is into the new object.
  */
  dbCtorHelper.normalizeArgs = function(filename=':memory:',flags = 'c',vfs = null){
    const arg = {};
    if(1===arguments.length && arguments[0] && 'object'===typeof arguments[0]){
      Object.assign(arg, arguments[0]);
      if(undefined===arg.flags) arg.flags = 'c';
      if(undefined===arg.vfs) arg.vfs = null;
      if(undefined===arg.filename) arg.filename = ':memory:';
    }else{
      arg.filename = filename;
      arg.flags = flags;
      arg.vfs = vfs;
    }
    return arg;
  };
  /**
     The DB class provides a high-level OO wrapper around an sqlite3
     db handle.

     The given db filename must be resolvable using whatever
     filesystem layer (virtual or otherwise) is set up for the default
     sqlite3 VFS or a VFS which can resolve it must be specified.

     The special sqlite3 db names ":memory:" and "" (temporary db)
     have their normal special meanings here and need not resolve to
     real filenames, but "" uses an on-storage temporary database and
     requires that the VFS support that.

     The second argument specifies the open/create mode for the
     database. It must be string containing a sequence of letters (in
     any order, but case sensitive) specifying the mode:

     - "c": create if it does not exist, else fail if it does not
       exist. Implies the "w" flag.

     - "w": write. Implies "r": a db cannot be write-only.

     - "r": read-only if neither "w" nor "c" are provided, else it
       is ignored.

     - "t": enable tracing of SQL executed on this database handle,
       sending it to `console.log()`. To disable it later, call
       `sqlite3.capi.sqlite3_trace_v2(thisDb.pointer, 0, 0, 0)`.

     If "w" is not provided, the db is implicitly read-only, noting
     that "rc" is meaningless

     Any other letters are currently ignored. The default is
     "c". These modes are ignored for the special ":memory:" and ""
     names and _may_ be ignored altogether for certain VFSes.

     The final argument is analogous to the final argument of
     sqlite3_open_v2(): the name of an sqlite3 VFS. Pass a falsy value,
     or none at all, to use the default. If passed a value, it must
     be the string name of a VFS.

     The constructor optionally (and preferably) takes its arguments
     in the form of a single configuration object with the following
     properties:

     - `filename`: database file name
     - `flags`: open-mode flags
     - `vfs`: the VFS fname


     The `filename` and `vfs` arguments may be either JS strings or
     C-strings allocated via WASM. `flags` is required to be a JS
     string (because it's specific to this API, which is specific
     to JS).

     For purposes of passing a DB instance to C-style sqlite3
     functions, the DB object's read-only `pointer` property holds its
     `sqlite3*` pointer value. That property can also be used to check
     whether this DB instance is still open: it will evaluate to
     `undefined` after the DB object's close() method is called.

     In the main window thread, the filenames `":localStorage:"` and
     `":sessionStorage:"` are special: they cause the db to use either
     localStorage or sessionStorage for storing the database using
     the kvvfs. If one of these names are used, they trump
     any vfs name set in the arguments.
  */
  const DB = function(...args){
    dbCtorHelper.apply(this, args);
  };
  DB.dbCtorHelper = dbCtorHelper;

  /**
     Internal-use enum for mapping JS types to DB-bindable types.
     These do not (and need not) line up with the SQLITE_type
     values. All values in this enum must be truthy and (mostly)
     distinct but they need not be numbers.
  */
  const BindTypes = {
    null: 1,
    number: 2,
    string: 3,
    boolean: 4,
    blob: 5
  };
  if(wasm.bigIntEnabled){
    BindTypes.bigint = BindTypes.number;
  }

  /**
     This class wraps sqlite3_stmt. Calling this constructor
     directly will trigger an exception. Use DB.prepare() to create
     new instances.

     For purposes of passing a Stmt instance to C-style sqlite3
     functions, its read-only `pointer` property holds its `sqlite3_stmt*`
     pointer value.

     Other non-function properties include:

     - `db`: the DB object which created the statement.

     - `columnCount`: the number of result columns in the query, or 0
     for queries which cannot return results. This property is a
     read-only proxy for sqlite3_column_count() and its use in loops
     should be avoided because of the call overhead associated with
     that. The `columnCount` is not cached when the Stmt is created
     because a schema change made between this statement's preparation
     and when it is stepped may invalidate it.

     - `parameterCount`: the number of bindable parameters in the
     query.  Like `columnCount`, this property is ready-only and is a
     proxy for a C API call.

     As a general rule, most methods of this class will throw if
     called on an instance which has been finalized. For brevity's
     sake, the method docs do not all repeat this warning.
  */
  const Stmt = function(/*oo1db, stmtPtr, BindTypes [,takeOwnership=true] */){
    if(BindTypes!==arguments[2]){
      toss3(capi.SQLITE_MISUSE, "Do not call the Stmt constructor directly. Use DB.prepare().");
    }
    this.db = arguments[0];
    __ptrMap.set(this, arguments[1]);
    if( arguments.length>3 && !arguments[3] ){
      __doesNotOwnHandle.add(this);
    }
  };

  /** Throws if the given DB has been closed, else it is returned. */
  const affirmDbOpen = function(db){
    if(!db.pointer) toss3("DB has been closed.");
    return db;
  };

  /** Throws if ndx is not an integer or if it is out of range
      for stmt.columnCount, else returns stmt.

      Reminder: this will also fail after the statement is finalized
      but the resulting error will be about an out-of-bounds column
      index rather than a statement-is-finalized error.
  */
  const affirmColIndex = function(stmt,ndx){
    if((ndx !== (ndx|0)) || ndx<0 || ndx>=stmt.columnCount){
      toss3("Column index",ndx,"is out of range.");
    }
    return stmt;
  };

  /**
     Expects to be passed the `arguments` object from DB.exec(). Does
     the argument processing/validation, throws on error, and returns
     a new object on success:

     { sql: the SQL, opt: optionsObj, cbArg: function}

     The opt object is a normalized copy of any passed to this
     function. The sql will be converted to a string if it is provided
     in one of the supported non-string formats.

     cbArg is only set if the opt.callback or opt.resultRows are set,
     in which case it's a function which expects to be passed the
     current Stmt and returns the callback argument of the type
     indicated by the input arguments.
  */
  const parseExecArgs = function(db, args){
    const out = Object.create(null);
    out.opt = Object.create(null);
    switch(args.length){
        case 1:
          if('string'===typeof args[0] || util.isSQLableTypedArray(args[0])){
            out.sql = args[0];
          }else if(Array.isArray(args[0])){
            out.sql = args[0];
          }else if(args[0] && 'object'===typeof args[0]){
            out.opt = args[0];
            out.sql = out.opt.sql;
          }
          break;
        case 2:
          out.sql = args[0];
          out.opt = args[1];
          break;
        default: toss3("Invalid argument count for exec().");
    };
    out.sql = util.flexibleString(out.sql);
    if('string'!==typeof out.sql){
      toss3("Missing SQL argument or unsupported SQL value type.");
    }
    const opt = out.opt;
    switch(opt.returnValue){
        case 'resultRows':
          if(!opt.resultRows) opt.resultRows = [];
          out.returnVal = ()=>opt.resultRows;
          break;
        case 'saveSql':
          if(!opt.saveSql) opt.saveSql = [];
          out.returnVal = ()=>opt.saveSql;
          break;
        case undefined:
        case 'this':
          out.returnVal = ()=>db;
          break;
        default:
          toss3("Invalid returnValue value:",opt.returnValue);
    }
    if(!opt.callback && !opt.returnValue && undefined!==opt.rowMode){
      if(!opt.resultRows) opt.resultRows = [];
      out.returnVal = ()=>opt.resultRows;
    }
    if(opt.callback || opt.resultRows){
      switch((undefined===opt.rowMode) ? 'array' : opt.rowMode) {
        case 'object':
          out.cbArg = (stmt,cache)=>{
            if( !cache.columnNames ) cache.columnNames = stmt.getColumnNames([]);
            /* https://sqlite.org/forum/forumpost/3632183d2470617d:
               conversion of rows to objects (key/val pairs) is
               somewhat expensive for large data sets because of the
               native-to-JS conversion of the column names. If we
               instead cache the names and build objects from that
               list of strings, it can run twice as fast. The
               difference is not noticeable for small data sets but
               becomes human-perceivable when enough rows are
               involved. */
            const row = stmt.get([]);
            const rv = Object.create(null);
            for( const i in cache.columnNames ) rv[cache.columnNames[i]] = row[i];
            return rv;
          };
          break;
        case 'array': out.cbArg = (stmt)=>stmt.get([]); break;
        case 'stmt':
          if(Array.isArray(opt.resultRows)){
            toss3("exec(): invalid rowMode for a resultRows array: must",
                  "be one of 'array', 'object',",
                  "a result column number, or column name reference.");
          }
          out.cbArg = (stmt)=>stmt;
          break;
        default:
          if(util.isInt32(opt.rowMode)){
            out.cbArg = (stmt)=>stmt.get(opt.rowMode);
            break;
          }else if('string'===typeof opt.rowMode
                   && opt.rowMode.length>1
                   && '$'===opt.rowMode[0]){
            /* "$X": fetch column named "X" (case-sensitive!). Prior
               to 2022-12-14 ":X" and "@X" were also permitted, but
               having so many options is unnecessary and likely to
               cause confusion. */
            const $colName = opt.rowMode.substr(1);
            out.cbArg = (stmt)=>{
              const rc = stmt.get(Object.create(null))[$colName];
              return (undefined===rc)
                ? toss3(capi.SQLITE_NOTFOUND,
                        "exec(): unknown result column:",$colName)
                : rc;
            };
            break;
          }
          toss3("Invalid rowMode:",opt.rowMode);
      }
    }
    return out;
  };

  /**
     Internal impl of the DB.selectValue(), selectArray(), and
     selectObject() methods.
  */
  const __selectFirstRow = (db, sql, bind, ...getArgs)=>{
    const stmt = db.prepare(sql);
    try {
      const rc = stmt.bind(bind).step() ? stmt.get(...getArgs) : undefined;
      stmt.reset(/*for INSERT...RETURNING locking case*/);
      return rc;
    }finally{
      stmt.finalize();
    }
  };

  /**
     Internal impl of the DB.selectArrays() and selectObjects()
     methods.
  */
  const __selectAll =
        (db, sql, bind, rowMode)=>db.exec({
          sql, bind, rowMode, returnValue: 'resultRows'
        });

  /**
     Expects to be given a DB instance or an `sqlite3*` pointer (may
     be null) and an sqlite3 API result code. If the result code is
     not falsy, this function throws an SQLite3Error with an error
     message from sqlite3_errmsg(), using db (or, if db is-a DB,
     db.pointer) as the db handle, or sqlite3_errstr() if db is
     falsy. Note that if it's passed a non-error code like SQLITE_ROW
     or SQLITE_DONE, it will still throw but the error string might be
     "Not an error."  The various non-0 non-error codes need to be
     checked for in client code where they are expected.

     The thrown exception's `resultCode` property will be the value of
     the second argument to this function.

     If it does not throw, it returns its first argument.
  */
  DB.checkRc = (db,resultCode)=>checkSqlite3Rc(db,resultCode);

  DB.prototype = {
    /** Returns true if this db handle is open, else false. */
    isOpen: function(){
      return !!this.pointer;
    },
    /** Throws if this given DB has been closed, else returns `this`. */
    affirmOpen: function(){
      return affirmDbOpen(this);
    },
    /**
       Finalizes all open statements and closes this database
       connection (with one exception noted below). This is a no-op if
       the db has already been closed. After calling close(),
       `this.pointer` will resolve to `undefined`, and that can be
       used to check whether the db instance is still opened.

       If this.onclose.before is a function then it is called before
       any close-related cleanup.

       If this.onclose.after is a function then it is called after the
       db is closed but before auxiliary state like this.filename is
       cleared.

       Both onclose handlers are passed this object, with the onclose
       object as their "this," noting that the db will have been
       closed when onclose.after is called. If this db is not opened
       when close() is called, neither of the handlers are called. Any
       exceptions the handlers throw are ignored because "destructors
       must not throw".

       Garbage collection of a db handle, if it happens at all, will
       never trigger close(), so onclose handlers are not a reliable
       way to implement close-time cleanup or maintenance of a db.

       If this instance was created using DB.wrapHandle() and does not
       own this.pointer then it does not close the db handle but it
       does perform all other work, such as calling onclose callbacks
       and disassociating this object from this.pointer.
    */
    close: function(){
      const pDb = this.pointer;
      if(pDb){
        if(this.onclose && (this.onclose.before instanceof Function)){
          try{this.onclose.before(this)}
          catch(e){/*ignore*/}
        }
        Object.keys(__stmtMap.get(this)).forEach((k,s)=>{
          if(s && s.pointer){
            try{s.finalize()}
            catch(e){/*ignore*/}
          }
        });
        __ptrMap.delete(this);
        __stmtMap.delete(this);
        if( !__doesNotOwnHandle.delete(this) ){
          capi.sqlite3_close_v2(pDb);
        }
        if(this.onclose && (this.onclose.after instanceof Function)){
          try{this.onclose.after(this)}
          catch(e){/*ignore*/}
        }
        delete this.filename;
      }
    },
    /**
       Returns the number of changes, as per sqlite3_changes()
       (if the first argument is false) or sqlite3_total_changes()
       (if it's true). If the 2nd argument is true, it uses
       sqlite3_changes64() or sqlite3_total_changes64(), which
       will trigger an exception if this build does not have
       BigInt support enabled.
    */
    changes: function(total=false,sixtyFour=false){
      const p = affirmDbOpen(this).pointer;
      if(total){
        return sixtyFour
          ? capi.sqlite3_total_changes64(p)
          : capi.sqlite3_total_changes(p);
      }else{
        return sixtyFour
          ? capi.sqlite3_changes64(p)
          : capi.sqlite3_changes(p);
      }
    },
    /**
       Similar to the this.filename but returns the
       sqlite3_db_filename() value for the given database name,
       defaulting to "main".  The argument may be either a JS string
       or a pointer to a WASM-allocated C-string.

       this.filename may be in the form of a URI-style string, whereas
       the returned string contains only the filename part.
    */
    dbFilename: function(dbName='main'){
      return capi.sqlite3_db_filename(affirmDbOpen(this).pointer, dbName);
    },
    /**
       Returns the name of the given 0-based db number, as documented
       for sqlite3_db_name().
    */
    dbName: function(dbNumber=0){
      return capi.sqlite3_db_name(affirmDbOpen(this).pointer, dbNumber);
    },
    /**
       Returns the name of the sqlite3_vfs used by the given database
       of this connection (defaulting to 'main'). The argument may be
       either a JS string or a WASM C-string. Returns undefined if the
       given db name is invalid. Throws if this object has been
       close()d.
    */
    dbVfsName: function(dbName=0){
      let rc;
      const pVfs = capi.sqlite3_js_db_vfs(
        affirmDbOpen(this).pointer, dbName
      );
      if(pVfs){
        const v = new capi.sqlite3_vfs(pVfs);
        try{ rc = wasm.cstrToJs(v.$zName) }
        finally { v.dispose() }
      }
      return rc;
    },
    /**
       Compiles the given SQL and returns a prepared Stmt. This is
       the only way to create new Stmt objects. Throws on error.

       The given SQL must be a string, a Uint8Array holding SQL, a
       WASM pointer to memory holding the NUL-terminated SQL string,
       or an array of strings. In the latter case, the array is
       concatenated together, with no separators, to form the SQL
       string (arrays are often a convenient way to formulate long
       statements).  If the SQL contains no statements, an
       SQLite3Error is thrown.

       Design note: the C API permits empty SQL, reporting it as a 0
       result code and a NULL stmt pointer. Supporting that case here
       would cause extra work for all clients: any use of the Stmt API
       on such a statement will necessarily throw, so clients would be
       required to check `stmt.pointer` after calling `prepare()` in
       order to determine whether the Stmt instance is empty or not.
       Long-time practice (with other sqlite3 script bindings)
       suggests that the empty-prepare case is sufficiently rare that
       supporting it here would simply hurt overall usability.
    */
    prepare: function(sql){
      affirmDbOpen(this);
      const stack = wasm.pstack.pointer;
      let ppStmt, pStmt;
      try{
        ppStmt = wasm.pstack.alloc(8)/* output (sqlite3_stmt**) arg */;
        DB.checkRc(this, capi.sqlite3_prepare_v2(this.pointer, sql, -1, ppStmt, null));
        pStmt = wasm.peekPtr(ppStmt);
      }
      finally {
        wasm.pstack.restore(stack);
      }
      if(!pStmt) toss3("Cannot prepare empty SQL.");
      const stmt = new Stmt(this, pStmt, BindTypes);
      __stmtMap.get(this)[pStmt] = stmt;
      return stmt;
    },
    /**
       Executes one or more SQL statements in the form of a single
       string. Its arguments must be either (sql,optionsObject) or
       (optionsObject). In the latter case, optionsObject.sql must
       contain the SQL to execute. By default it returns this object
       but that can be changed via the `returnValue` option as
       described below. Throws on error.

       If no SQL is provided, or a non-string is provided, an
       exception is triggered. Empty SQL, on the other hand, is
       simply a no-op.

       The optional options object may contain any of the following
       properties:

       - `sql` = the SQL to run (unless it's provided as the first
       argument). This must be of type string, Uint8Array, or an array
       of strings. In the latter case they're concatenated together
       as-is, _with no separator_ between elements, before evaluation.
       The array form is often simpler for long hand-written queries.

       - `bind` = a single value valid as an argument for
       Stmt.bind(). This is _only_ applied to the _first_ non-empty
       statement in the SQL which has any bindable parameters. (Empty
       statements are skipped entirely.)

       - `saveSql` = an optional array. If set, the SQL of each
       executed statement is appended to this array before the
       statement is executed (but after it is prepared - we don't have
       the string until after that). Empty SQL statements are elided
       but can have odd effects in the output. e.g. SQL of: `"select
       1; -- empty\n; select 2"` will result in an array containing
       `["select 1;", "--empty \n; select 2"]`. That's simply how
       sqlite3 records the SQL for the 2nd statement.

       ==================================================================
       The following options apply _only_ to the _first_ statement
       which has a non-zero result column count, regardless of whether
       the statement actually produces any result rows.
       ==================================================================

       - `columnNames`: if this is an array, the column names of the
       result set are stored in this array before the callback (if
       any) is triggered (regardless of whether the query produces any
       result rows). If no statement has result columns, this value is
       unchanged. Achtung: an SQL result may have multiple columns
       with identical names.

       - `callback` = a function which gets called for each row of the
       result set, but only if that statement has any result rows. The
       callback's "this" is the options object, noting that this
       function synthesizes one if the caller does not pass one to
       exec(). The first argument passed to the callback is described
       below. The second argument is always the current Stmt object,
       as it's needed if the caller wants to fetch the column names or
       some such (noting that they could also be fetched via
       `this.columnNames`, if the client provides the `columnNames`
       option). If the callback returns a literal `false` (as opposed
       to any other falsy value, e.g. an implicit `undefined` return),
       any ongoing statement-`step()` iteration stops without an
       error. The return value of the callback is otherwise ignored.

       ACHTUNG: The callback MUST NOT modify the Stmt object. Calling
       any of the Stmt.get() variants, Stmt.getColumnName(), or
       similar, is legal, but calling step() or finalize() is
       not. Member methods which are illegal in this context will
       trigger an exception, but clients must also refrain from using
       any lower-level (C-style) APIs which might modify the
       statement.

       The first argument passed to the callback defaults to an array of
       values from the current result row but may be changed with ...

       - `rowMode` = specifies the type of he callback's first argument.
       It may be any of...

       A) A string describing what type of argument should be passed
       as the first argument to the callback:

         A.1) `'array'` (the default) causes the results of
         `stmt.get([])` to be passed to the `callback` and/or appended
         to `resultRows`.

         A.2) `'object'` causes the results of
         `stmt.get(Object.create(null))` to be passed to the
         `callback` and/or appended to `resultRows`.  Achtung: an SQL
         result may have multiple columns with identical names. In
         that case, the right-most column will be the one set in this
         object!

         A.3) `'stmt'` causes the current Stmt to be passed to the
         callback, but this mode will trigger an exception if
         `resultRows` is an array because appending the transient
         statement to the array would be downright unhelpful.  This
         option is a legacy feature, retained for backwards
         compatibility.  The statement object is passed as the second
         argument to the callback, as described above.

       B) An integer, indicating a zero-based column in the result
       row. Only that one single value, in JS form, will be passed on.

       C) A string with a minimum length of 2 and leading character of
       '$' will fetch the row as an object, extract that one field,
       and pass that field's value to the callback. These keys are
       case-sensitive so must match the case used in the
       SQL. e.g. `"select a A from t"` with a `rowMode` of `'$A'`
       would work but `'$a'` would not. A reference to a column not in
       the result set will trigger an exception on the first row (as
       the check is not performed until rows are fetched).  Note that
       `$` is a legal identifier character in JS so need not be
       quoted.

       Any other `rowMode` value triggers an exception.

       - `resultRows`: if this is an array, it functions similarly to
       the `callback` option: each row of the result set (if any),
       with the exception that the `rowMode` 'stmt' is not legal. It
       is legal to use both `resultRows` and `callback`, but
       `resultRows` is likely much simpler to use for small data sets
       and can be used over a WebWorker-style message interface.
       exec() throws if `resultRows` is set and `rowMode` is 'stmt'.

       - `returnValue`: is a string specifying what this function
       should return:

         A) The default value is (usually) `"this"`, meaning that the
            DB object itself should be returned. The exception is if
            the caller passes neither of `callback` nor `returnValue`
            but does pass an explicit `rowMode` then the default
            `returnValue` is `"resultRows"`, described below.

         B) `"resultRows"` means to return the value of the
            `resultRows` option. If `resultRows` is not set, this
            function behaves as if it were set to an empty array.

         C) `"saveSql"` means to return the value of the
            `saveSql` option. If `saveSql` is not set, this
            function behaves as if it were set to an empty array.

       Potential TODOs:

       - `bind`: permit an array of arrays/objects to bind. The first
       sub-array would act on the first statement which has bindable
       parameters (as it does now). The 2nd would act on the next such
       statement, etc.

       - `callback` and `resultRows`: permit an array entries with
       semantics similar to those described for `bind` above.

       OTOH, this function already does too much.
    */
    exec: function(/*(sql [,obj]) || (obj)*/){
      affirmDbOpen(this);
      const arg = parseExecArgs(this, arguments);
      if(!arg.sql){
        return toss3("exec() requires an SQL string.");
      }
      const opt = arg.opt;
      const callback = opt.callback;
      const resultRows =
            Array.isArray(opt.resultRows) ? opt.resultRows : undefined;
      let stmt;
      let bind = opt.bind;
      let evalFirstResult = !!(
        arg.cbArg || opt.columnNames || resultRows
      ) /* true to step through the first result-returning statement */;
      const stack = wasm.scopedAllocPush();
      const saveSql = Array.isArray(opt.saveSql) ? opt.saveSql : undefined;
      try{
        const isTA = util.isSQLableTypedArray(arg.sql)
        /* Optimization: if the SQL is a TypedArray we can save some string
           conversion costs. */;
        /* Allocate the two output pointers (ppStmt, pzTail) and heap
           space for the SQL (pSql). When prepare_v3() returns, pzTail
           will point to somewhere in pSql. */
        let sqlByteLen = isTA ? arg.sql.byteLength : wasm.jstrlen(arg.sql);
        const ppStmt  = wasm.scopedAlloc(
          /* output (sqlite3_stmt**) arg and pzTail */
          (2 * wasm.ptr.size) + (sqlByteLen + 1/* SQL + NUL */)
        );
        const pzTail = wasm.ptr.add(ppStmt, wasm.ptr.size) /* final arg to sqlite3_prepare_v2() */;
        let pSql = wasm.ptr.add(pzTail, wasm.ptr.size);
        const pSqlEnd = wasm.ptr.add(pSql, sqlByteLen);
        if(isTA) wasm.heap8().set(arg.sql, pSql);
        else wasm.jstrcpy(arg.sql, wasm.heap8(), pSql, sqlByteLen, false);
        wasm.poke8(wasm.ptr.add(pSql, sqlByteLen), 0/*NUL terminator*/);
        while(pSql && wasm.peek8(pSql)
              /* Maintenance reminder:^^^ _must_ be 'i8' or else we
                 will very likely cause an endless loop. What that's
                 doing is checking for a terminating NUL byte. If we
                 use i32 or similar then we read 4 bytes, read stuff
                 around the NUL terminator, and get stuck in and
                 endless loop at the end of the SQL, endlessly
                 re-preparing an empty statement. */ ){
          wasm.pokePtr([ppStmt, pzTail], 0);
          DB.checkRc(this, capi.sqlite3_prepare_v3(
            this.pointer, pSql, sqlByteLen, 0, ppStmt, pzTail
          ));
          const pStmt = wasm.peekPtr(ppStmt);
          pSql = wasm.peekPtr(pzTail);
          sqlByteLen = Number(wasm.ptr.add(pSqlEnd,-pSql));
          if(!pStmt) continue;
          //sqlite3.config.debug("exec() pSql =",capi.sqlite3_sql(pStmt));
          if(saveSql) saveSql.push(capi.sqlite3_sql(pStmt).trim());
          stmt = new Stmt(this, pStmt, BindTypes);
          if(bind && stmt.parameterCount){
            stmt.bind(bind);
            bind = null;
          }
          if(evalFirstResult && stmt.columnCount){
            /* Only forward SELECT-style results for the FIRST query
               in the SQL which potentially has them. */
            let gotColNames = Array.isArray(
              opt.columnNames
              /* As reported in
                 https://sqlite.org/forum/forumpost/7774b773937cbe0a
                 we need to delay fetching of the column names until
                 after the first step() (if we step() at all) because
                 a schema change between the prepare() and step(), via
                 another connection, may invalidate the column count
                 and names. */) ? 0 : 1;
            evalFirstResult = false;
            if(arg.cbArg || resultRows){
              const cbArgCache = Object.create(null)
              /* 2nd arg for arg.cbArg, used by (at least) row-to-object
                 converter */;
              for( ; stmt.step(); __execLock.delete(stmt) ){
                if(0===gotColNames++){
                  stmt.getColumnNames(cbArgCache.columnNames = (opt.columnNames || []));
                }
                __execLock.add(stmt);
                const row = arg.cbArg(stmt,cbArgCache);
                if(resultRows) resultRows.push(row);
                if(callback && false === callback.call(opt, row, stmt)){
                  break;
                }
              }
              __execLock.delete(stmt);
            }
            if(0===gotColNames){
              /* opt.columnNames was provided but we visited no result rows */
              stmt.getColumnNames(opt.columnNames);
            }
          }else{
            stmt.step();
          }
          stmt.reset(
            /* In order to trigger an exception in the
               INSERT...RETURNING locking scenario:
               https://sqlite.org/forum/forumpost/36f7a2e7494897df
               [tag:insert-returning-reset]
            */).finalize();
          stmt = null;
        }/*prepare() loop*/
      }/*catch(e){
        sqlite3.config.warn("DB.exec() is propagating exception",opt,e);
        throw e;
      }*/finally{
        if(stmt){
          __execLock.delete(stmt);
          stmt.finalize();
        }
        wasm.scopedAllocPop(stack);
      }
      return arg.returnVal();
    }/*exec()*/,


    /**
       Creates a new UDF (User-Defined Function) which is accessible
       via SQL code. This function may be called in any of the
       following forms:

       - (name, function)
       - (name, function, optionsObject)
       - (name, optionsObject)
       - (optionsObject)

       In the final two cases, the function must be defined as the
       `callback` property of the options object (optionally called
       `xFunc` to align with the C API documentation). In the final
       case, the function's name must be the 'name' property.

       The first two call forms can only be used for creating scalar
       functions. Creating an aggregate or window function requires
       the options-object form (see below for details).

       UDFs can be removed as documented for
       sqlite3_create_function_v2() and
       sqlite3_create_window_function(), but doing so will "leak" the
       JS-created WASM binding of those functions (meaning that their
       entries in the WASM indirect function table still
       exist). Eliminating that potential leak is a pending TODO.

       On success, returns this object. Throws on error.

       When called from SQL arguments to the UDF, and its result,
       will be converted between JS and SQL with as much fidelity as
       is feasible, triggering an exception if a type conversion
       cannot be determined. The docs for sqlite3_create_function_v2()
       describe the conversions in more detail.

       The values set in the options object differ for scalar and
       aggregate functions:

       - Scalar: set the `xFunc` function-type property to the UDF
         function.

       - Aggregate: set the `xStep` and `xFinal` function-type
         properties to the "step" and "final" callbacks for the
         aggregate. Do not set the `xFunc` property.

       - Window: set the `xStep`, `xFinal`, `xValue`, and `xInverse`
         function-type properties. Do not set the `xFunc` property.

       The options object may optionally have an `xDestroy`
       function-type property, as per sqlite3_create_function_v2().
       Its argument will be the WASM-pointer-type value of the `pApp`
       property, and this function will throw if `pApp` is defined but
       is not null, undefined, or a numeric (WASM pointer)
       value. i.e. `pApp`, if set, must be value suitable for use as a
       WASM pointer argument, noting that `null` or `undefined` will
       translate to 0 for that purpose.

       The options object may contain flags to modify how
       the function is defined:

       - `arity`: the number of arguments which SQL calls to this
       function expect or require. The default value is `xFunc.length`
       or `xStep.length` (i.e. the number of declared parameters it
       has) **MINUS 1** (see below for why). As a special case, if the
       `length` is 0, its arity is also 0 instead of -1. A negative
       arity value means that the function is variadic and may accept
       any number of arguments, up to sqlite3's compile-time
       limits. sqlite3 will enforce the argument count if is zero or
       greater. The callback always receives a pointer to an
       `sqlite3_context` object as its first argument. Any arguments
       after that are from SQL code. The leading context argument does
       _not_ count towards the function's arity. See the docs for
       sqlite3.capi.sqlite3_create_function_v2() for why that argument
       is needed in the interface.

       The following options-object properties correspond to flags
       documented at:

       https://sqlite.org/c3ref/create_function.html

       - `deterministic` = sqlite3.capi.SQLITE_DETERMINISTIC
       - `directOnly` = sqlite3.capi.SQLITE_DIRECTONLY
       - `innocuous` = sqlite3.capi.SQLITE_INNOCUOUS

       Sidebar: the ability to add new WASM-accessible functions to
       the runtime requires that the WASM build is compiled with the
       equivalent functionality as that provided by Emscripten's
       `-sALLOW_TABLE_GROWTH` flag.
    */
    createFunction: function f(name, xFunc, opt){
      const isFunc = (f)=>(f instanceof Function);
      switch(arguments.length){
          case 1: /* (optionsObject) */
            opt = name;
            name = opt.name;
            xFunc = opt.xFunc || 0;
            break;
          case 2: /* (name, callback|optionsObject) */
            if(!isFunc(xFunc)){
              opt = xFunc;
              xFunc = opt.xFunc || 0;
            }
            break;
          case 3: /* name, xFunc, opt */
            break;
          default: break;
      }
      if(!opt) opt = {};
      if('string' !== typeof name){
        toss3("Invalid arguments: missing function name.");
      }
      let xStep = opt.xStep || 0;
      let xFinal = opt.xFinal || 0;
      const xValue = opt.xValue || 0;
      const xInverse = opt.xInverse || 0;
      let isWindow = undefined;
      if(isFunc(xFunc)){
        isWindow = false;
        if(isFunc(xStep) || isFunc(xFinal)){
          toss3("Ambiguous arguments: scalar or aggregate?");
        }
        xStep = xFinal = null;
      }else if(isFunc(xStep)){
        if(!isFunc(xFinal)){
          toss3("Missing xFinal() callback for aggregate or window UDF.");
        }
        xFunc = null;
      }else if(isFunc(xFinal)){
        toss3("Missing xStep() callback for aggregate or window UDF.");
      }else{
        toss3("Missing function-type properties.");
      }
      if(false === isWindow){
        if(isFunc(xValue) || isFunc(xInverse)){
          toss3("xValue and xInverse are not permitted for non-window UDFs.");
        }
      }else if(isFunc(xValue)){
        if(!isFunc(xInverse)){
          toss3("xInverse must be provided if xValue is.");
        }
        isWindow = true;
      }else if(isFunc(xInverse)){
        toss3("xValue must be provided if xInverse is.");
      }
      const pApp = opt.pApp;
      if( undefined!==pApp
          && null!==pApp
          && !wasm.isPtr(pApp) ){
        toss3("Invalid value for pApp property. Must be a legal WASM pointer value.");
      }
      const xDestroy = opt.xDestroy || 0;
      if(xDestroy && !isFunc(xDestroy)){
        toss3("xDestroy property must be a function.");
      }
      let fFlags = 0 /*flags for sqlite3_create_function_v2()*/;
      if(getOwnOption(opt, 'deterministic')) fFlags |= capi.SQLITE_DETERMINISTIC;
      if(getOwnOption(opt, 'directOnly')) fFlags |= capi.SQLITE_DIRECTONLY;
      if(getOwnOption(opt, 'innocuous')) fFlags |= capi.SQLITE_INNOCUOUS;
      name = name.toLowerCase();
      const xArity = xFunc || xStep;
      const arity = getOwnOption(opt, 'arity');
      const arityArg = ('number'===typeof arity
                        ? arity
                        : (xArity.length ? xArity.length-1/*for pCtx arg*/ : 0));
      let rc;
      if( isWindow ){
        rc = capi.sqlite3_create_window_function(
          this.pointer, name, arityArg,
          capi.SQLITE_UTF8 | fFlags, pApp || 0,
          xStep, xFinal, xValue, xInverse, xDestroy);
      }else{
        rc = capi.sqlite3_create_function_v2(
          this.pointer, name, arityArg,
          capi.SQLITE_UTF8 | fFlags, pApp || 0,
          xFunc, xStep, xFinal, xDestroy);
      }
      DB.checkRc(this, rc);
      return this;
    }/*createFunction()*/,
    /**
       Prepares the given SQL, step()s it one time, and returns
       the value of the first result column. If it has no results,
       undefined is returned.

       If passed a second argument, it is treated like an argument
       to Stmt.bind(), so may be any type supported by that
       function. Passing the undefined value is the same as passing
       no value, which is useful when...

       If passed a 3rd argument, it is expected to be one of the
       SQLITE_{typename} constants. Passing the undefined value is
       the same as not passing a value.

       Throws on error (e.g. malformed SQL).
    */
    selectValue: function(sql,bind,asType){
      return __selectFirstRow(this, sql, bind, 0, asType);
    },

    /**
       Runs the given query and returns an array of the values from
       the first result column of each row of the result set. The 2nd
       argument is an optional value for use in a single-argument call
       to Stmt.bind(). The 3rd argument may be any value suitable for
       use as the 2nd argument to Stmt.get(). If a 3rd argument is
       desired but no bind data are needed, pass `undefined` for the 2nd
       argument.

       If there are no result rows, an empty array is returned.
    */
    selectValues: function(sql,bind,asType){
      const stmt = this.prepare(sql), rc = [];
      try {
        stmt.bind(bind);
        while(stmt.step()) rc.push(stmt.get(0,asType));
        stmt.reset(/*for INSERT...RETURNING locking case*/);
      }finally{
        stmt.finalize();
      }
      return rc;
    },

    /**
       Prepares the given SQL, step()s it one time, and returns an
       array containing the values of the first result row. If it has
       no results, `undefined` is returned.

       If passed a second argument other than `undefined`, it is
       treated like an argument to Stmt.bind(), so may be any type
       supported by that function.

       Throws on error (e.g. malformed SQL).
    */
    selectArray: function(sql,bind){
      return __selectFirstRow(this, sql, bind, []);
    },

    /**
       Prepares the given SQL, step()s it one time, and returns an
       object containing the key/value pairs of the first result
       row. If it has no results, `undefined` is returned.

       Note that the order of returned object's keys is not guaranteed
       to be the same as the order of the fields in the query string.

       If passed a second argument other than `undefined`, it is
       treated like an argument to Stmt.bind(), so may be any type
       supported by that function.

       Throws on error (e.g. malformed SQL).
    */
    selectObject: function(sql,bind){
      return __selectFirstRow(this, sql, bind, {});
    },

    /**
       Runs the given SQL and returns an array of all results, with
       each row represented as an array, as per the 'array' `rowMode`
       option to `exec()`. An empty result set resolves
       to an empty array. The second argument, if any, is treated as
       the 'bind' option to a call to exec().
    */
    selectArrays: function(sql,bind){
      return __selectAll(this, sql, bind, 'array');
    },

    /**
       Works identically to selectArrays() except that each value
       in the returned array is an object, as per the 'object' `rowMode`
       option to `exec()`.
    */
    selectObjects: function(sql,bind){
      return __selectAll(this, sql, bind, 'object');
    },

    /**
       Returns the number of currently-opened Stmt handles for this db
       handle, or 0 if this DB instance is closed. Note that only
       handles prepared via this.prepare() are counted, and not
       handles prepared using capi.sqlite3_prepare_v3() (or
       equivalent).
    */
    openStatementCount: function(){
      return this.pointer ? Object.keys(__stmtMap.get(this)).length : 0;
    },

    /**
       Starts a transaction, calls the given callback, and then either
       rolls back or commits the transaction, depending on whether the
       callback throws. The callback is passed this db object as its
       only argument. On success, returns the result of the
       callback. Throws on error.

       Transactions may not be nested, so this will throw if it is
       called recursively. For nested transactions, use the
       savepoint() method or manually manage SAVEPOINTs using exec().

       If called with 2 arguments, the first must be a keyword which
       is legal immediately after a BEGIN statement, e.g. one of
       "DEFERRED", "IMMEDIATE", or "EXCLUSIVE", though the exact list
       of supported keywords is not hard-coded here, in order to be
       future-compatible, if the argument does not look like a single
       keyword then an exception is triggered with a description of
       the problem.
    */
    transaction: function(/* [beginQualifier,] */callback){
      let opener = 'BEGIN';
      if(arguments.length>1){
        if(/[^a-zA-Z]/.test(arguments[0])){
          toss3(capi.SQLITE_MISUSE, "Invalid argument for BEGIN qualifier.");
        }
        opener += ' '+arguments[0];
        callback = arguments[1];
      }
      affirmDbOpen(this).exec(opener);
      try {
        const rc = callback(this);
        this.exec("COMMIT");
        return rc;
      }catch(e){
        this.exec("ROLLBACK");
        throw e;
      }
    },

    /**
       This works similarly to transaction() but uses sqlite3's SAVEPOINT
       feature. This function starts a savepoint (with an unspecified name)
       and calls the given callback function, passing it this db object.
       If the callback returns, the savepoint is released (committed). If
       the callback throws, the savepoint is rolled back. If it does not
       throw, it returns the result of the callback.
    */
    savepoint: function(callback){
      affirmDbOpen(this).exec("SAVEPOINT oo1");
      try {
        const rc = callback(this);
        this.exec("RELEASE oo1");
        return rc;
      }catch(e){
        this.exec("ROLLBACK to SAVEPOINT oo1; RELEASE SAVEPOINT oo1");
        throw e;
      }
    },

    /**
       A convenience form of DB.checkRc(this,resultCode). If it does
       not throw, it returns this object.
    */
    checkRc: function(resultCode){
      return checkSqlite3Rc(this, resultCode);
    },
  }/*DB.prototype*/;

  /**
     Returns a new oo1.DB instance which wraps the given (sqlite3*)
     WASM pointer, optionally with or without taking over ownership of
     that pointer.

     The first argument must be either a non-NULL (sqlite3*) WASM
     pointer.

     The second argument, defaulting to false, specifies ownership of
     the first argument. If it is truthy, the returned object will
     pass that pointer to sqlite3_close() when its close() method is
     called, otherwise it will not.

     Throws if pDb is not a non-0 WASM pointer.

     The caller MUST GUARANTEE that the passed-in handle will outlive
     the returned object, i.e. that it will not be closed. If it is closed,
     this object will hold a stale pointer and results are undefined.

     Aside from its lifetime, the proxy is to be treated as any other
     DB instance, including the requirement of calling close() on
     it. close() will free up internal resources owned by the proxy
     and disassociate the proxy from that handle but will not
     actually close the proxied db handle unless this function is
     passed a thruthy second argument.

     To stress:

     - DO NOT call sqlite3_close() (or similar) on the being-proxied
       pointer while a proxy is active.

     - ALWAYS eventually call close() on the returned object. If the
       proxy does not own the underlying handle then its MUST be
       closed BEFORE the being-proxied handle is closed.

     Design notes:

     - wrapHandle() "could" accept a DB object instance as its first
       argument and proxy thatDb.pointer but there is currently no use
       case where doing so would be useful, so it does not allow
       that. That restriction may be lifted in a future version.
  */
  DB.wrapHandle = function(pDb, takeOwnership=false){
    if( !pDb || !wasm.isPtr(pDb) ){
      throw new sqlite3.SQLite3Error(capi.SQLITE_MISUSE,
                                     "Argument must be a WASM sqlite3 pointer");
    }
    return new DB({
      /* This ctor call style is very specifically internal-use-only.
         It is not documented and may change at any time. */
      "sqlite3*": pDb,
      "sqlite3*:takeOwnership": !!takeOwnership
    });
  };

  /** Throws if the given Stmt has been finalized, else stmt is
      returned. */
  const affirmStmtOpen = function(stmt){
    if(!stmt.pointer) toss3("Stmt has been closed.");
    return stmt;
  };

  /** Returns an opaque truthy value from the BindTypes
      enum if v's type is a valid bindable type, else
      returns a falsy value. As a special case, a value of
      undefined is treated as a bind type of null. */
  const isSupportedBindType = function(v){
    let t = BindTypes[(null===v||undefined===v) ? 'null' : typeof v];
    switch(t){
        case BindTypes.boolean:
        case BindTypes.null:
        case BindTypes.number:
        case BindTypes.string:
          return t;
        case BindTypes.bigint:
          return wasm.bigIntEnabled ? t : undefined;
        default:
          return util.isBindableTypedArray(v) ? BindTypes.blob : undefined;
    }
  };

  /**
     If isSupportedBindType(v) returns a truthy value, this
     function returns that value, else it throws.
  */
  const affirmSupportedBindType = function(v){
    //sqlite3.config.log('affirmSupportedBindType',v);
    return isSupportedBindType(v) || toss3("Unsupported bind() argument type:",typeof v);
  };

  /**
     If key is a number and within range of stmt's bound parameter
     count, key is returned.

     If key is not a number then it must be a JS string (not a WASM
     string) and it is checked against named parameters. If a match is
     found, its index is returned.

     Else it throws.
  */
  const affirmParamIndex = function(stmt,key){
    const n = ('number'===typeof key)
          ? key : capi.sqlite3_bind_parameter_index(stmt.pointer, key);
    if( 0===n || !util.isInt32(n) ) toss3("Invalid bind() parameter name: "+key);
    else if( n<1 || n>stmt.parameterCount ) toss3("Bind index",key,"is out of range.");
    return n;
  };

  /**
     Each Stmt object which is "locked" by DB.exec() gets an entry
     here to note that "lock".

     The reason this is in place is because exec({callback:...})'s
     callback gets access to the Stmt objects created internally by
     exec() but it must not use certain Stmt APIs.
  */
  const __execLock = new Set();
  /**
     This is a Stmt.get() counterpart of __execLock. Each time
     Stmt.step() returns true, the statement is added to this set,
     indicating that Stmt.get() is legal. Stmt APIs which invalidate
     that status remove the Stmt object from this set, which will
     cause Stmt.get() to throw with a descriptive error message
     instead of a more generic "API misuse" if we were to allow that
     call to reach the C API.
  */
  const __stmtMayGet = new Set();

  /**
     Stmt APIs which are prohibited on locked objects must call
     affirmNotLockedByExec() before doing any work.

     If __execLock.has(stmt) is truthy, this throws an exception
     complaining that the 2nd argument (an operation name,
     e.g. "bind()") is not legal while the statement is "locked".
     Locking happens before an exec()-like callback is passed a
     statement, to ensure that the callback does not mutate or
     finalize the statement. If it does not throw, it returns stmt.
  */
  const affirmNotLockedByExec = function(stmt,currentOpName){
    if(__execLock.has(stmt)){
      toss3("Operation is illegal when statement is locked:",currentOpName);
    }
    return stmt;
  };

  /**
     Binds a single bound parameter value on the given stmt at the
     given index (numeric or named) using the given bindType (see
     the BindTypes enum) and value. Throws on error. Returns stmt on
     success.
  */
  const bindOne = function f(stmt,ndx,bindType,val){
    affirmNotLockedByExec(affirmStmtOpen(stmt), 'bind()');
    if(!f._){
      f._tooBigInt = (v)=>toss3(
        "BigInt value is too big to store without precision loss:", v
      );
      f._ = {
        string: function(stmt, ndx, val, asBlob){
          const [pStr, n] = wasm.allocCString(val, true);
          const f = asBlob ? capi.sqlite3_bind_blob : capi.sqlite3_bind_text;
          return f(stmt.pointer, ndx, pStr, n, capi.SQLITE_WASM_DEALLOC);
        }
      };
    }/* static init */
    affirmSupportedBindType(val);
    ndx = affirmParamIndex(stmt,ndx);
    let rc = 0;
    switch((null===val || undefined===val) ? BindTypes.null : bindType){
        case BindTypes.null:
          rc = capi.sqlite3_bind_null(stmt.pointer, ndx);
          break;
        case BindTypes.string:
          rc = f._.string(stmt, ndx, val, false);
          break;
        case BindTypes.number: {
          let m;
          if(util.isInt32(val)) m = capi.sqlite3_bind_int;
          else if('bigint'===typeof val){
            if(!util.bigIntFits64(val)){
              f._tooBigInt(val);
            }else if(wasm.bigIntEnabled){
              m = capi.sqlite3_bind_int64;
            }else if(util.bigIntFitsDouble(val)){
              val = Number(val);
              m = capi.sqlite3_bind_double;
            }else{
              f._tooBigInt(val);
            }
          }else{ // !int32, !bigint
            val = Number(val);
            if(wasm.bigIntEnabled && Number.isInteger(val)){
              m = capi.sqlite3_bind_int64;
            }else{
              m = capi.sqlite3_bind_double;
            }
          }
          rc = m(stmt.pointer, ndx, val);
          break;
        }
        case BindTypes.boolean:
          rc = capi.sqlite3_bind_int(stmt.pointer, ndx, val ? 1 : 0);
          break;
        case BindTypes.blob: {
          if('string'===typeof val){
            rc = f._.string(stmt, ndx, val, true);
            break;
          }else if(val instanceof ArrayBuffer){
            val = new Uint8Array(val);
          }else if(!util.isBindableTypedArray(val)){
            toss3("Binding a value as a blob requires",
                  "that it be a string, Uint8Array, Int8Array, or ArrayBuffer.");
          }
          const pBlob = wasm.alloc(val.byteLength || 1);
          wasm.heap8().set(val.byteLength ? val : [0], Number(pBlob))
          rc = capi.sqlite3_bind_blob(stmt.pointer, ndx, pBlob, val.byteLength,
                                      capi.SQLITE_WASM_DEALLOC);
          break;
        }
        default:
          sqlite3.config.warn("Unsupported bind() argument type:",val);
          toss3("Unsupported bind() argument type: "+(typeof val));
    }
    if(rc) DB.checkRc(stmt.db.pointer, rc);
    return stmt;
  };

  Stmt.prototype = {
    /**
       "Finalizes" this statement. This is a no-op if the statement
       has already been finalized. Returns the result of
       sqlite3_finalize() (0 on success, non-0 on error), or the
       undefined value if the statement has already been
       finalized. Regardless of success or failure, most methods in
       this class will throw if called after this is.

       This method always throws if called when it is illegal to do
       so. Namely, when triggered via a per-row callback handler of a
       DB.exec() call.

       If Stmt does not own its underlying (sqlite3_stmt*) (see
       Stmt.wrapHandle()) then this function will not pass it to
       sqlite3_finalize().
    */
    finalize: function(){
      const ptr = this.pointer;
      if(ptr){
        affirmNotLockedByExec(this,'finalize()');
        const rc = (__doesNotOwnHandle.delete(this)
                    ? 0
                    : capi.sqlite3_finalize(ptr));
        delete __stmtMap.get(this.db)[ptr];
        __ptrMap.delete(this);
        __execLock.delete(this);
        __stmtMayGet.delete(this);
        delete this.parameterCount;
        delete this.db;
        return rc;
      }
    },
    /**
       Clears all bound values. Returns this object.  Throws if this
       statement has been finalized or if modification of the
       statement is currently illegal (e.g. in the per-row callback of
       a DB.exec() call).
    */
    clearBindings: function(){
      affirmNotLockedByExec(affirmStmtOpen(this), 'clearBindings()')
      capi.sqlite3_clear_bindings(this.pointer);
        __stmtMayGet.delete(this);
      return this;
    },
    /**
       Resets this statement so that it may be step()ed again from the
       beginning. Returns this object. Throws if this statement has
       been finalized, if it may not legally be reset because it is
       currently being used from a DB.exec() callback, or if the
       underlying call to sqlite3_reset() returns non-0.

       If passed a truthy argument then this.clearBindings() is
       also called, otherwise any existing bindings, along with
       any memory allocated for them, are retained.

       In versions 3.42.0 and earlier, this function did not throw if
       sqlite3_reset() returns non-0, but it was discovered that
       throwing (or significant extra client-side code) is necessary
       in order to avoid certain silent failure scenarios, as
       discussed at:

       https://sqlite.org/forum/forumpost/36f7a2e7494897df
    */
    reset: function(alsoClearBinds){
      affirmNotLockedByExec(this,'reset()');
      if(alsoClearBinds) this.clearBindings();
      const rc = capi.sqlite3_reset(affirmStmtOpen(this).pointer);
      __stmtMayGet.delete(this);
      checkSqlite3Rc(this.db, rc);
      return this;
    },
    /**
       Binds one or more values to its bindable parameters. It
       accepts 1 or 2 arguments:

       If passed a single argument, it must be either an array, an
       object, or a value of a bindable type (see below).

       If passed 2 arguments, the first one is the 1-based bind
       index or bindable parameter name and the second one must be
       a value of a bindable type.

       Bindable value types:

       - null is bound as NULL.

       - undefined as a standalone value is a no-op intended to
         simplify certain client-side use cases: passing undefined as
         a value to this function will not actually bind anything and
         this function will skip confirmation that binding is even
         legal. (Those semantics simplify certain client-side uses.)
         Conversely, a value of undefined as an array or object
         property when binding an array/object (see below) is treated
         the same as null.

       - Numbers are bound as either doubles or integers: doubles if
         they are larger than 32 bits, else double or int32, depending
         on whether they have a fractional part. Booleans are bound as
         integer 0 or 1. It is not expected the distinction of binding
         doubles which have no fractional parts and integers is
         significant for the majority of clients due to sqlite3's data
         typing model. If BigInt support is enabled then this routine
         will bind BigInt values as 64-bit integers if they'll fit in
         64 bits. If that support disabled, it will store the BigInt
         as an int32 or a double if it can do so without loss of
         precision. If the BigInt is _too BigInt_ then it will throw.

       - Strings are bound as strings (use bindAsBlob() to force
         blob binding).

       - Uint8Array, Int8Array, and ArrayBuffer instances are bound as
         blobs.

       If passed an array, each element of the array is bound at
       the parameter index equal to the array index plus 1
       (because arrays are 0-based but binding is 1-based).

       If passed an object, each object key is treated as a
       bindable parameter name. The object keys _must_ match any
       bindable parameter names, including any `$`, `@`, or `:`
       prefix. Because `$` is a legal identifier chararacter in
       JavaScript, that is the suggested prefix for bindable
       parameters: `stmt.bind({$a: 1, $b: 2})`.

       It returns this object on success and throws on
       error. Errors include:

       - Any bind index is out of range, a named bind parameter
       does not match, or this statement has no bindable
       parameters.

       - Any value to bind is of an unsupported type.

       - Passed no arguments or more than two.

       - The statement has been finalized.
    */
    bind: function(/*[ndx,] arg*/){
      affirmStmtOpen(this);
      let ndx, arg;
      switch(arguments.length){
          case 1: ndx = 1; arg = arguments[0]; break;
          case 2: ndx = arguments[0]; arg = arguments[1]; break;
          default: toss3("Invalid bind() arguments.");
      }
      if(undefined===arg){
        /* It might seem intuitive to bind undefined as NULL
           but this approach simplifies certain client-side
           uses when passing on arguments between 2+ levels of
           functions. */
        return this;
      }else if(!this.parameterCount){
        toss3("This statement has no bindable parameters.");
      }
      __stmtMayGet.delete(this);
      if(null===arg){
        /* bind NULL */
        return bindOne(this, ndx, BindTypes.null, arg);
      }
      else if(Array.isArray(arg)){
        /* bind each entry by index */
        if(1!==arguments.length){
          toss3("When binding an array, an index argument is not permitted.");
        }
        arg.forEach((v,i)=>bindOne(this, i+1, affirmSupportedBindType(v), v));
        return this;
      }else if(arg instanceof ArrayBuffer){
        arg = new Uint8Array(arg);
      }
      if('object'===typeof arg/*null was checked above*/
              && !util.isBindableTypedArray(arg)){
        /* Treat each property of arg as a named bound parameter. */
        if(1!==arguments.length){
          toss3("When binding an object, an index argument is not permitted.");
        }
        Object.keys(arg)
          .forEach(k=>bindOne(this, k,
                              affirmSupportedBindType(arg[k]),
                              arg[k]));
        return this;
      }else{
        return bindOne(this, ndx, affirmSupportedBindType(arg), arg);
      }
      toss3("Should not reach this point.");
    },
    /**
       Special case of bind() which binds the given value using the
       BLOB binding mechanism instead of the default selected one for
       the value. The ndx may be a numbered or named bind index. The
       value must be of type string, null/undefined (both get treated
       as null), or a TypedArray of a type supported by the bind()
       API. This API cannot bind numbers as blobs.

       If passed a single argument, a bind index of 1 is assumed and
       the first argument is the value.
    */
    bindAsBlob: function(ndx,arg){
      affirmStmtOpen(this);
      if(1===arguments.length){
        arg = ndx;
        ndx = 1;
      }
      const t = affirmSupportedBindType(arg);
      if(BindTypes.string !== t && BindTypes.blob !== t
         && BindTypes.null !== t){
        toss3("Invalid value type for bindAsBlob()");
      }
      return bindOne(this, ndx, BindTypes.blob, arg);
    },
    /**
       Steps the statement one time. If the result indicates that a
       row of data is available, a truthy value is returned.  If no
       row of data is available, a falsy value is returned.  Throws on
       error.
    */
    step: function(){
      affirmNotLockedByExec(this, 'step()');
      const rc = capi.sqlite3_step(affirmStmtOpen(this).pointer);
      switch(rc){
        case capi.SQLITE_DONE:
          __stmtMayGet.delete(this);
          return false;
        case capi.SQLITE_ROW:
          __stmtMayGet.add(this);
          return true;
        default:
          __stmtMayGet.delete(this);
          sqlite3.config.warn("sqlite3_step() rc=",rc,
                              capi.sqlite3_js_rc_str(rc),
                              "SQL =", capi.sqlite3_sql(this.pointer));
          DB.checkRc(this.db.pointer, rc);
      }
    },
    /**
       Functions exactly like step() except that...

       1) On success, it calls this.reset() and returns this object.

       2) On error, it throws and does not call reset().

       This is intended to simplify constructs like:

       ```
       for(...) {
         stmt.bind(...).stepReset();
       }
       ```

       Note that the reset() call makes it illegal to call this.get()
       after the step.
    */
    stepReset: function(){
      this.step();
      return this.reset();
    },
    /**
       Functions like step() except that it calls finalize() on this
       statement immediately after stepping, even if the step() call
       throws.

       On success, it returns true if the step indicated that a row of
       data was available, else it returns a falsy value.

       This is intended to simplify use cases such as:

       ```
       aDb.prepare("insert into foo(a) values(?)").bind(123).stepFinalize();
       ```
    */
    stepFinalize: function(){
      try{
        const rc = this.step();
        this.reset(/*for INSERT...RETURNING locking case*/);
        return rc;
      }finally{
        try{this.finalize()}
        catch(e){/*ignored*/}
      }
    },

    /**
       Fetches the value from the given 0-based column index of
       the current data row, throwing if index is out of range.

       Requires that step() has just returned a truthy value, else
       an exception is thrown.

       By default it will determine the data type of the result
       automatically. If passed a second argument, it must be one
       of the enumeration values for sqlite3 types, which are
       defined as members of the sqlite3 module: SQLITE_INTEGER,
       SQLITE_FLOAT, SQLITE_TEXT, SQLITE_BLOB. Any other value,
       except for undefined, will trigger an exception. Passing
       undefined is the same as not passing a value. It is legal
       to, e.g., fetch an integer value as a string, in which case
       sqlite3 will convert the value to a string.

       If ndx is an array, this function behaves a differently: it
       assigns the indexes of the array, from 0 to the number of
       result columns, to the values of the corresponding column,
       and returns that array.

       If ndx is a plain object, this function behaves even
       differentlier: it assigns the properties of the object to
       the values of their corresponding result columns and returns
       that object.

       Blobs are returned as Uint8Array instances.

       Potential TODO: add type ID SQLITE_JSON, which fetches the
       result as a string and passes it (if it's not null) to
       JSON.parse(), returning the result of that. Until then,
       getJSON() can be used for that.
    */
    get: function(ndx,asType){
      if(!__stmtMayGet.has(affirmStmtOpen(this))){
        toss3("Stmt.step() has not (recently) returned true.");
      }
      if(Array.isArray(ndx)){
        let i = 0;
        const n = this.columnCount;
        while(i<n){
          ndx[i] = this.get(i++);
        }
        return ndx;
      }else if(ndx && 'object'===typeof ndx){
        let i = 0;
        const n = this.columnCount;
        while(i<n){
          ndx[capi.sqlite3_column_name(this.pointer,i)] = this.get(i++);
        }
        return ndx;
      }
      affirmColIndex(this, ndx);
      switch(undefined===asType
             ? capi.sqlite3_column_type(this.pointer, ndx)
             : asType){
          case capi.SQLITE_NULL: return null;
          case capi.SQLITE_INTEGER:{
            if(wasm.bigIntEnabled){
              const rc = capi.sqlite3_column_int64(this.pointer, ndx);
              if(rc>=Number.MIN_SAFE_INTEGER && rc<=Number.MAX_SAFE_INTEGER){
                /* Coerce "normal" number ranges to normal number values,
                   and only return BigInt-type values for numbers out of this
                   range. */
                return Number(rc).valueOf();
              }
              return rc;
            }else{
              const rc = capi.sqlite3_column_double(this.pointer, ndx);
              if(rc>Number.MAX_SAFE_INTEGER || rc<Number.MIN_SAFE_INTEGER){
                /* Throwing here is arguable but, since we're explicitly
                   extracting an SQLITE_INTEGER-type value, it seems fair to throw
                   if the extracted number is out of range for that type.
                   This policy may be laxened to simply pass on the number and
                   hope for the best, as the C API would do. */
                toss3("Integer is out of range for JS integer range: "+rc);
              }
              //sqlite3.config.log("get integer rc=",rc,isInt32(rc));
              return util.isInt32(rc) ? (rc | 0) : rc;
            }
          }
          case capi.SQLITE_FLOAT:
            return capi.sqlite3_column_double(this.pointer, ndx);
          case capi.SQLITE_TEXT:
            return capi.sqlite3_column_text(this.pointer, ndx);
          case capi.SQLITE_BLOB: {
            const n = capi.sqlite3_column_bytes(this.pointer, ndx),
                  ptr = capi.sqlite3_column_blob(this.pointer, ndx),
                  rc = new Uint8Array(n);
            if(n){
              rc.set(wasm.heap8u().slice(Number(ptr), Number(ptr)+n), 0);
              if(this.db._blobXfer instanceof Array){
                /* This is an optimization soley for the Worker1 API. It
                   will transfer these to the main thread directly
                   instead of copying them. */
                this.db._blobXfer.push(rc.buffer);
              }
            }
            return rc;
          }
          default: toss3("Don't know how to translate",
                         "type of result column #"+ndx+".");
      }
      toss3("Not reached.");
    },
    /** Equivalent to get(ndx) but coerces the result to an
        integer. */
    getInt: function(ndx){return this.get(ndx,capi.SQLITE_INTEGER)},
    /** Equivalent to get(ndx) but coerces the result to a
        float. */
    getFloat: function(ndx){return this.get(ndx,capi.SQLITE_FLOAT)},
    /** Equivalent to get(ndx) but coerces the result to a
        string. */
    getString: function(ndx){return this.get(ndx,capi.SQLITE_TEXT)},
    /** Equivalent to get(ndx) but coerces the result to a
        Uint8Array. */
    getBlob: function(ndx){return this.get(ndx,capi.SQLITE_BLOB)},
    /**
       A convenience wrapper around get() which fetches the value
       as a string and then, if it is not null, passes it to
       JSON.parse(), returning that result. Throws if parsing
       fails. If the result is null, null is returned. An empty
       string, on the other hand, will trigger an exception.
    */
    getJSON: function(ndx){
      const s = this.get(ndx, capi.SQLITE_STRING);
      return null===s ? s : JSON.parse(s);
    },
    // Design note: the only reason most of these getters have a 'get'
    // prefix is for consistency with getVALUE_TYPE().  The latter
    // arguably really need that prefix for API readability and the
    // rest arguably don't, but consistency is a powerful thing.
    /**
       Returns the result column name of the given index, or
       throws if index is out of bounds or this statement has been
       finalized. This can be used without having run step()
       first.
    */
    getColumnName: function(ndx){
      return capi.sqlite3_column_name(
        affirmColIndex(affirmStmtOpen(this),ndx).pointer, ndx
      );
    },
    /**
       If this statement potentially has result columns, this function
       returns an array of all such names. If passed an array, it is
       used as the target and all names are appended to it. Returns
       the target array. Throws if this statement cannot have result
       columns. This object's columnCount property holds the number of
       columns.
    */
    getColumnNames: function(tgt=[]){
      affirmColIndex(affirmStmtOpen(this),0);
      const n = this.columnCount;
      for(let i = 0; i < n; ++i){
        tgt.push(capi.sqlite3_column_name(this.pointer, i));
      }
      return tgt;
    },
    /**
       If this statement has named bindable parameters and the
       given name matches one, its 1-based bind index is
       returned. If no match is found, 0 is returned. If it has no
       bindable parameters, the undefined value is returned.
    */
    getParamIndex: function(name){
      return (affirmStmtOpen(this).parameterCount
              ? capi.sqlite3_bind_parameter_index(this.pointer, name)
              : undefined);
    },
    /**
       If this statement has named bindable parameters and the given
       index refers to one, its name is returned, else null is
       returned. If this statement has no bound parameters, undefined
       is returned.

       Added in 3.47.
    */
    getParamName: function(ndx){
      return (affirmStmtOpen(this).parameterCount
              ? capi.sqlite3_bind_parameter_name(this.pointer, ndx)
              : undefined);
    },

    /**
       Behaves like sqlite3_stmt_busy() but throws if this statement
       is closed and returns a value of type boolean instead of integer.

       Added in 3.47.
    */
    isBusy: function(){
      return 0!==capi.sqlite3_stmt_busy(affirmStmtOpen(this));
    },

    /**
       Behaves like sqlite3_stmt_readonly() but throws if this statement
       is closed and returns a value of type boolean instead of integer.

       Added in 3.47.
    */
    isReadOnly: function(){
      return 0!==capi.sqlite3_stmt_readonly(affirmStmtOpen(this));
    }
  }/*Stmt.prototype*/;

  {/* Add the `pointer` property to DB and Stmt. */
    const prop = {
      enumerable: true,
      get: function(){return __ptrMap.get(this)},
      set: ()=>toss3("The pointer property is read-only.")
    }
    Object.defineProperty(Stmt.prototype, 'pointer', prop);
    Object.defineProperty(DB.prototype, 'pointer', prop);
  }
  /**
     Stmt.columnCount is an interceptor for sqlite3_column_count().

     This requires an unfortunate performance hit compared to caching
     columnCount when the Stmt is created/prepared (as was done in
     SQLite <=3.42.0), but is necessary in order to handle certain
     corner cases, as described in
     https://sqlite.org/forum/forumpost/7774b773937cbe0a.
  */
  Object.defineProperty(Stmt.prototype, 'columnCount', {
    enumerable: false,
    get: function(){return capi.sqlite3_column_count(this.pointer)},
    set: ()=>toss3("The columnCount property is read-only.")
  });

  Object.defineProperty(Stmt.prototype, 'parameterCount', {
    enumerable: false,
    get: function(){return capi.sqlite3_bind_parameter_count(this.pointer)},
    set: ()=>toss3("The parameterCount property is read-only.")
  });

  /**
     The Stmt counterpart of oo1.DB.wrapHandle(), this creates a Stmt
     instance which wraps a WASM (sqlite3_stmt*) in the oo1 API,
     optionally with or without taking over ownership of that pointer.

     The first argument must be an oo1.DB instance[^1].

     The second argument must be a valid WASM (sqlite3_stmt*), as
     produced by sqlite3_prepare_v2() and sqlite3_prepare_v3().

     The third argument, defaulting to false, specifies whether the
     returned Stmt object takes over ownership of the underlying
     (sqlite3_stmt*). If true, the returned object's finalize() method
     will finalize that handle, else it will not. If it is false,
     ownership of pStmt is unchanged and pStmt MUST outlive the
     returned object or results are undefined.

     This function throws if the arguments are invalid. On success it
     returns a new Stmt object which wraps the given statement
     pointer.

     Like all Stmt objects, the finalize() method must eventually be
     called on the returned object to free up internal resources,
     regardless of whether this function's third argument is true or
     not.

     [^1]: The first argument cannot be a (sqlite3*) because the
     resulting Stmt object requires a parent DB object. It is not yet
     determined whether it would be of general benefit to refactor the
     DB/Stmt pair internals to communicate in terms of the underlying
     (sqlite3*) rather than a DB object. If so, we could laxen the
     first argument's requirement and allow an (sqlite3*). Because
     DB.wrapHandle() enables multiple DB objects to proxy the same
     (sqlite3*), we cannot unambiguously translate the first arugment
     from (sqlite3*) to DB instances for us with this function's first
     argument.
  */
  Stmt.wrapHandle = function(oo1db, pStmt, takeOwnership=false){
    let ctor = Stmt;
    if( !(oo1db instanceof DB) || !oo1db.pointer ){
      throw new sqlite3.SQLite3Error(sqlite3.SQLITE_MISUSE,
                                     "First argument must be an opened "+
                                     "sqlite3.oo1.DB instance");
    }
    if( !pStmt || !wasm.isPtr(pStmt) ){
      throw new sqlite3.SQLite3Error(sqlite3.SQLITE_MISUSE,
                                     "Second argument must be a WASM "+
                                     "sqlite3_stmt pointer");
    }
    return new Stmt(oo1db, pStmt, BindTypes, !!takeOwnership);
  }

  /** The OO API's public namespace. */
  sqlite3.oo1 = {
    DB,
    Stmt
  }/*oo1 object*/;

});
/**
  2022-07-22

  The author disclaims copyright to this source code.  In place of a
  legal notice, here is a blessing:

  *   May you do good and not evil.
  *   May you find forgiveness for yourself and forgive others.
  *   May you share freely, never taking more than you give.

  ***********************************************************************

  This file implements the initializer for SQLite's "Worker API #1", a
  very basic DB access API intended to be scripted from a main window
  thread via Worker-style messages. Because of limitations in that
  type of communication, this API is minimalistic and only capable of
  serving relatively basic DB requests (e.g. it cannot process nested
  query loops concurrently).

  This file requires that the core C-style sqlite3 API and OO API #1
  have been loaded.
*/

/**
  sqlite3.initWorker1API() implements a Worker-based wrapper around
  SQLite3 OO API #1, colloquially known as "Worker API #1".

  In order to permit this API to be loaded in worker threads without
  automatically registering onmessage handlers, initializing the
  worker API requires calling initWorker1API(). If this function is
  called from a non-worker thread then it throws an exception.  It
  must only be called once per Worker.

  When initialized, it installs message listeners to receive Worker
  messages and then it posts a message in the form:

  ```
  {type:'sqlite3-api', result:'worker1-ready'}
  ```

  to let the client know that it has been initialized. Clients may
  optionally depend on this function not returning until
  initialization is complete, as the initialization is synchronous.
  In some contexts, however, listening for the above message is
  a better fit.

  Note that the worker-based interface can be slightly quirky because
  of its async nature. In particular, any number of messages may be posted
  to the worker before it starts handling any of them. If, e.g., an
  "open" operation fails, any subsequent messages will fail. The
  Promise-based wrapper for this API (`sqlite3-worker1-promiser.js`)
  is more comfortable to use in that regard.

  The documentation for the input and output worker messages for
  this API follows...

  ====================================================================
  Common message format...

  Each message posted to the worker has an operation-independent
  envelope and operation-dependent arguments:

  ```
  {
    type: string, // one of: 'open', 'close', 'exec', 'export', 'config-get'

    messageId: OPTIONAL arbitrary value. The worker will copy it as-is
    into response messages to assist in client-side dispatching.

    dbId: a db identifier string (returned by 'open') which tells the
    operation which database instance to work on. If not provided, the
    first-opened db is used. This is an "opaque" value, with no
    inherently useful syntax or information. Its value is subject to
    change with any given build of this API and cannot be used as a
    basis for anything useful beyond its one intended purpose.

    args: ...operation-dependent arguments...

    // the framework may add other properties for testing or debugging
    // purposes.

  }
  ```

  Response messages, posted back to the main thread, look like:

  ```
  {
    type: string. Same as above except for error responses, which have the type
    'error',

    messageId: same value, if any, provided by the inbound message

    dbId: the id of the db which was operated on, if any, as returned
    by the corresponding 'open' operation.

    result: ...operation-dependent result...

  }
  ```

  ====================================================================
  Error responses

  Errors are reported messages in an operation-independent format:

  ```
  {
    type: "error",

    messageId: ...as above...,

    dbId: ...as above...

    result: {

      operation: type of the triggering operation: 'open', 'close', ...

      message: ...error message text...

      errorClass: string. The ErrorClass.name property from the thrown exception.

      input: the message object which triggered the error.

      stack: _if available_, a stack trace array.

    }

  }
  ```


  ====================================================================
  "config-get"

  This operation fetches the serializable parts of the sqlite3 API
  configuration.

  Message format:

  ```
  {
    type: "config-get",
    messageId: ...as above...,
    args: currently ignored and may be elided.
  }
  ```

  Response:

  ```
  {
    type: "config-get",
    messageId: ...as above...,
    result: {

      version: sqlite3.version object

      bigIntEnabled: bool. True if BigInt support is enabled.

      vfsList: result of sqlite3.capi.sqlite3_js_vfs_list()
   }
  }
  ```


  ====================================================================
  "open" a database

  Message format:

  ```
  {
    type: "open",
    messageId: ...as above...,
    args:{

      filename [=":memory:" or "" (unspecified)]: the db filename.
      See the sqlite3.oo1.DB constructor for peculiarities and
      transformations,

      vfs: sqlite3_vfs name. Ignored if filename is ":memory:" or "".
           This may change how the given filename is resolved.
    }
  }
  ```

  Response:

  ```
  {
    type: "open",
    messageId: ...as above...,
    result: {
      filename: db filename, possibly differing from the input.

      dbId: an opaque ID value which must be passed in the message
      envelope to other calls in this API to tell them which db to
      use. If it is not provided to future calls, they will default to
      operating on the least-recently-opened db. This property is, for
      API consistency's sake, also part of the containing message
      envelope.  Only the `open` operation includes it in the `result`
      property.

      persistent: true if the given filename resides in the
      known-persistent storage, else false.

      vfs: name of the VFS the "main" db is using.
   }
  }
  ```

  ====================================================================
  "close" a database

  Message format:

  ```
  {
    type: "close",
    messageId: ...as above...
    dbId: ...as above...
    args: OPTIONAL {unlink: boolean}
  }
  ```

  If the `dbId` does not refer to an opened ID, this is a no-op. If
  the `args` object contains a truthy `unlink` value then the database
  will be unlinked (deleted) after closing it. The inability to close a
  db (because it's not opened) or delete its file does not trigger an
  error.

  Response:

  ```
  {
    type: "close",
    messageId: ...as above...,
    result: {

      filename: filename of closed db, or undefined if no db was closed

    }
  }
  ```

  ====================================================================
  "exec" SQL

  All SQL execution is processed through the exec operation. It offers
  most of the features of the oo1.DB.exec() method, with a few limitations
  imposed by the state having to cross thread boundaries.

  Message format:

  ```
  {
    type: "exec",
    messageId: ...as above...
    dbId: ...as above...
    args: string (SQL) or {... see below ...}
  }
  ```

  Response:

  ```
  {
    type: "exec",
    messageId: ...as above...,
    dbId: ...as above...
    result: {
      input arguments, possibly modified. See below.
    }
  }
  ```

  The arguments are in the same form accepted by oo1.DB.exec(), with
  the exceptions noted below.

  If `args.countChanges` (added in version 3.43) is truthy then the
  `result` property contained by the returned object will have a
  `changeCount` property which holds the number of changes made by the
  provided SQL. Because the SQL may contain an arbitrary number of
  statements, the `changeCount` is calculated by calling
  `sqlite3_total_changes()` before and after the SQL is evaluated. If
  the value of `countChanges` is 64 then the `changeCount` property
  will be returned as a 64-bit integer in the form of a BigInt (noting
  that that will trigger an exception if used in a BigInt-incapable
  build).  In the latter case, the number of changes is calculated by
  calling `sqlite3_total_changes64()` before and after the SQL is
  evaluated.

  If the `args.lastInsertRowId` (added in version 3.50.0) is truthy
  then the `result` property contained by the returned object will
  have a `lastInsertRowId` will hold a BigInt-type value corresponding
  to the result of sqlite3_last_insert_rowid(). This value is only
  fetched once, after the SQL is run, regardless of how many
  statements the SQL contains. This API has no idea whether the SQL
  contains any INSERTs, so it is up to the client to apply/rely on
  this property only when it makes sense to do so.

  A function-type args.callback property cannot cross
  the window/Worker boundary, so is not useful here. If
  args.callback is a string then it is assumed to be a
  message type key, in which case a callback function will be
  applied which posts each row result via:

  postMessage({type: thatKeyType,
               rowNumber: 1-based-#,
               row: theRow,
               columnNames: anArray
               })

  And, at the end of the result set (whether or not any result rows
  were produced), it will post an identical message with
  (row=undefined, rowNumber=null) to alert the caller than the result
  set is completed. Note that a row value of `null` is a legal row
  result for certain arg.rowMode values.

    (Design note: we don't use (row=undefined, rowNumber=undefined) to
    indicate end-of-results because fetching those would be
    indistinguishable from fetching from an empty object unless the
    client used hasOwnProperty() (or similar) to distinguish "missing
    property" from "property with the undefined value".  Similarly,
    `null` is a legal value for `row` in some case , whereas the db
    layer won't emit a result value of `undefined`.)

  The callback proxy must not recurse into this interface. An exec()
  call will tie up the Worker thread, causing any recursion attempt
  to wait until the first exec() is completed.

  The response is the input options object (or a synthesized one if
  passed only a string), noting that options.resultRows and
  options.columnNames may be populated by the call to db.exec().


  ====================================================================
  "export" the current db

  To export the underlying database as a byte array...

  Message format:

  ```
  {
    type: "export",
    messageId: ...as above...,
    dbId: ...as above...
  }
  ```

  Response:

  ```
  {
    type: "export",
    messageId: ...as above...,
    dbId: ...as above...
    result: {
      byteArray: Uint8Array (as per sqlite3_js_db_export()),
      filename: the db filename,
      mimetype: "application/x-sqlite3"
    }
  }
  ```

*/
globalThis.sqlite3ApiBootstrap.initializers.push(function(sqlite3){
const util = sqlite3.util;
sqlite3.initWorker1API = function(){
  'use strict';
  const toss = (...args)=>{throw new Error(args.join(' '))};
  if(!(globalThis.WorkerGlobalScope instanceof Function)){
    toss("initWorker1API() must be run from a Worker thread.");
  }
  const sqlite3 = this.sqlite3 || toss("Missing this.sqlite3 object.");
  const DB = sqlite3.oo1.DB;

  /**
     Returns the app-wide unique ID for the given db, creating one if
     needed.
  */
  const getDbId = function(db){
    let id = wState.idMap.get(db);
    if(id) return id;
    id = 'db#'+(++wState.idSeq)+':'+
      Math.floor(Math.random() * 100000000)+':'+
      Math.floor(Math.random() * 100000000);
    /** ^^^ can't simply use db.pointer b/c closing/opening may re-use
        the same address, which could map pending messages to a wrong
        instance.

        2025-07: https://github.com/sqlite/sqlite-wasm/issues/113
        demonstrates that two Worker1s can end up with the same IDs,
        despite using different instances of the library, so we need
        to add some randomness to the IDs instead of relying on the
        pointer addresses.
    */
    wState.idMap.set(db, id);
    return id;
  };

  /**
     Internal helper for managing Worker-level state.
  */
  const wState = {
    /**
       Each opened DB is added to this.dbList, and the first entry in
       that list is the default db. As each db is closed, its entry is
       removed from the list.
    */
    dbList: [],
    /** Sequence number of dbId generation. */
    idSeq: 0,
    /** Map of DB instances to dbId. */
    idMap: new WeakMap,
    /** Temp holder for "transferable" postMessage() state. */
    xfer: [],
    open: function(opt){
      const db = new DB(opt);
      this.dbs[getDbId(db)] = db;
      if(this.dbList.indexOf(db)<0) this.dbList.push(db);
      return db;
    },
    close: function(db,alsoUnlink){
      if(db){
        delete this.dbs[getDbId(db)];
        const filename = db.filename;
        const pVfs = util.sqlite3__wasm_db_vfs(db.pointer, 0);
        db.close();
        const ddNdx = this.dbList.indexOf(db);
        if(ddNdx>=0) this.dbList.splice(ddNdx, 1);
        if(alsoUnlink && filename && pVfs){
          util.sqlite3__wasm_vfs_unlink(pVfs, filename);
        }
      }
    },
    /**
       Posts the given worker message value. If xferList is provided,
       it must be an array, in which case a copy of it passed as
       postMessage()'s second argument and xferList.length is set to
       0.
    */
    post: function(msg,xferList){
      if(xferList && xferList.length){
        globalThis.postMessage( msg, Array.from(xferList) );
        xferList.length = 0;
      }else{
        globalThis.postMessage(msg);
      }
    },
    /** Map of DB IDs to DBs. */
    dbs: Object.create(null),
    /** Fetch the DB for the given id. Throw if require=true and the
        id is not valid, else return the db or undefined. */
    getDb: function(id,require=true){
      return this.dbs[id]
        || (require ? toss("Unknown (or closed) DB ID:",id) : undefined);
    }
  };

  /** Throws if the given db is falsy or not opened, else returns its
      argument. */
  const affirmDbOpen = function(db = wState.dbList[0]){
    return (db && db.pointer) ? db : toss("DB is not opened.");
  };

  /** Extract dbId from the given message payload. */
  const getMsgDb = function(msgData,affirmExists=true){
    const db = wState.getDb(msgData.dbId,false) || wState.dbList[0];
    return affirmExists ? affirmDbOpen(db) : db;
  };

  const getDefaultDbId = function(){
    return wState.dbList[0] && getDbId(wState.dbList[0]);
  };

  const isSpecialDbFilename = (n)=>{
    return ""===n || ':'===n[0];
  };

  /**
     A level of "organizational abstraction" for the Worker1
     API. Each method in this object must map directly to a Worker1
     message type key. The onmessage() dispatcher attempts to
     dispatch all inbound messages to a method of this object,
     passing it the event.data part of the inbound event object. All
     methods must return a plain Object containing any result
     state, which the dispatcher may amend. All methods must throw
     on error.
  */
  const wMsgHandler = {
    open: function(ev){
      const oargs = Object.create(null), args = (ev.args || Object.create(null));
      if(args.simulateError){ // undocumented internal testing option
        toss("Throwing because of simulateError flag.");
      }
      const rc = Object.create(null);
      oargs.vfs = args.vfs;
      oargs.filename = args.filename || "";
      const db = wState.open(oargs);
      rc.filename = db.filename;
      rc.persistent = !!sqlite3.capi.sqlite3_js_db_uses_vfs(db.pointer, "opfs");
      rc.dbId = getDbId(db);
      rc.vfs = db.dbVfsName();
      return rc;
    },

    close: function(ev){
      const db = getMsgDb(ev,false);
      const response = {
        filename: db && db.filename
      };
      if(db){
        const doUnlink = ((ev.args && 'object'===typeof ev.args)
                         ? !!ev.args.unlink : false);
        wState.close(db, doUnlink);
      }
      return response;
    },

    exec: function(ev){
      const rc = (
        'string'===typeof ev.args
      ) ? {sql: ev.args} : (ev.args || Object.create(null));
      if('stmt'===rc.rowMode){
        toss("Invalid rowMode for 'exec': stmt mode",
             "does not work in the Worker API.");
      }else if(!rc.sql){
        toss("'exec' requires input SQL.");
      }
      const db = getMsgDb(ev);
      if(rc.callback || Array.isArray(rc.resultRows)){
        // Part of a copy-avoidance optimization for blobs
        db._blobXfer = wState.xfer;
      }
      const theCallback = rc.callback;
      let rowNumber = 0;
      const hadColNames = !!rc.columnNames;
      if('string' === typeof theCallback){
        if(!hadColNames) rc.columnNames = [];
        /* Treat this as a worker message type and post each
           row as a message of that type. */
        rc.callback = function(row,stmt){
          wState.post({
            type: theCallback,
            columnNames: rc.columnNames,
            rowNumber: ++rowNumber,
            row: row
          }, wState.xfer);
        }
      }
      try {
        const changeCount = !!rc.countChanges
              ? db.changes(true,(64===rc.countChanges))
              : undefined;
        db.exec(rc);
        if(undefined !== changeCount){
          rc.changeCount = db.changes(true,64===rc.countChanges) - changeCount;
        }
        const lastInsertRowId = !!rc.lastInsertRowId
              ? sqlite3.capi.sqlite3_last_insert_rowid(db)
              : undefined;
        if( undefined!==lastInsertRowId ){
          rc.lastInsertRowId = lastInsertRowId;
        }
        if(rc.callback instanceof Function){
          rc.callback = theCallback;
          /* Post a sentinel message to tell the client that the end
             of the result set has been reached (possibly with zero
             rows). */
          wState.post({
            type: theCallback,
            columnNames: rc.columnNames,
            rowNumber: null /*null to distinguish from "property not set"*/,
            row: undefined /*undefined because null is a legal row value
                             for some rowType values, but undefined is not*/
          });
        }
      }finally{
        delete db._blobXfer;
        if(rc.callback) rc.callback = theCallback;
      }
      return rc;
    }/*exec()*/,

    'config-get': function(){
      const rc = Object.create(null), src = sqlite3.config;
      [
        'bigIntEnabled'
      ].forEach(function(k){
        if(Object.getOwnPropertyDescriptor(src, k)) rc[k] = src[k];
      });
      rc.version = sqlite3.version;
      rc.vfsList = sqlite3.capi.sqlite3_js_vfs_list();
      return rc;
    },

    /**
       Exports the database to a byte array, as per
       sqlite3_serialize(). Response is an object:

       {
         byteArray:  Uint8Array (db file contents),
         filename: the current db filename,
         mimetype: 'application/x-sqlite3'
       }
    */
    export: function(ev){
      const db = getMsgDb(ev);
      const response = {
        byteArray: sqlite3.capi.sqlite3_js_db_export(db.pointer),
        filename: db.filename,
        mimetype: 'application/x-sqlite3'
      };
      wState.xfer.push(response.byteArray.buffer);
      return response;
    }/*export()*/,

    toss: function(ev){
      toss("Testing worker exception");
    }
  }/*wMsgHandler*/;

  globalThis.onmessage = async function(ev){
    ev = ev.data;
    let result, dbId = ev.dbId, evType = ev.type;
    const arrivalTime = performance.now();
    try {
      if(wMsgHandler.hasOwnProperty(evType) &&
         wMsgHandler[evType] instanceof Function){
        result = await wMsgHandler[evType](ev);
      }else{
        toss("Unknown db worker message type:",ev.type);
      }
    }catch(err){
      evType = 'error';
      result = {
        operation: ev.type,
        message: err.message,
        errorClass: err.name,
        input: ev
      };
      if(err.stack){
        result.stack = ('string'===typeof err.stack)
          ? err.stack.split(/\n\s*/) : err.stack;
      }
      if(0) sqlite3.config.warn("Worker is propagating an exception to main thread.",
                                "Reporting it _here_ for the stack trace:",err,result);
    }
    if(!dbId){
      dbId = result.dbId/*from 'open' cmd*/
        || getDefaultDbId();
    }
    // Timing info is primarily for use in testing this API. It's not part of
    // the public API. arrivalTime = when the worker got the message.
    wState.post({
      type: evType,
      dbId: dbId,
      messageId: ev.messageId,
      workerReceivedTime: arrivalTime,
      workerRespondTime: performance.now(),
      departureTime: ev.departureTime,
      // TODO: move the timing bits into...
      //timing:{
      //  departure: ev.departureTime,
      //  workerReceived: arrivalTime,
      //  workerResponse: performance.now();
      //},
      result: result
    }, wState.xfer);
  };
  globalThis.postMessage({type:'sqlite3-api',result:'worker1-ready'});
}.bind({sqlite3});
});
/*
** 2022-11-30
**
** The author disclaims copyright to this source code.  In place of a
** legal notice, here is a blessing:
**
** *   May you do good and not evil.
** *   May you find forgiveness for yourself and forgive others.
** *   May you share freely, never taking more than you give.
*/

/**
   This file installs sqlite3.vfs, a namespace of helpers for use in
   the creation of JavaScript implementations of sqlite3_vfs.
*/
'use strict';
globalThis.sqlite3ApiBootstrap.initializers.push(function(sqlite3){
  const wasm = sqlite3.wasm, capi = sqlite3.capi, toss = sqlite3.util.toss3;
  const vfs = Object.create(null);
  sqlite3.vfs = vfs;

  /**
     Uses sqlite3_vfs_register() to register this
     sqlite3.capi.sqlite3_vfs instance. This object must have already
     been filled out properly. If the first argument is truthy, the
     VFS is registered as the default VFS, else it is not.

     On success, returns this object. Throws on error.
  */
  capi.sqlite3_vfs.prototype.registerVfs = function(asDefault=false){
    if(!(this instanceof sqlite3.capi.sqlite3_vfs)){
      toss("Expecting a sqlite3_vfs-type argument.");
    }
    const rc = capi.sqlite3_vfs_register(this, asDefault ? 1 : 0);
    if(rc){
      toss("sqlite3_vfs_register(",this,") failed with rc",rc);
    }
    if(this.pointer !== capi.sqlite3_vfs_find(this.$zName)){
      toss("BUG: sqlite3_vfs_find(vfs.$zName) failed for just-installed VFS",
           this);
    }
    return this;
  };

  /**
     A wrapper for
     sqlite3.StructBinder.StructType.prototype.installMethods() or
     registerVfs() to reduce installation of a VFS and/or its I/O
     methods to a single call.

     Accepts an object which contains the properties "io" and/or
     "vfs", each of which is itself an object with following properties:

     - `struct`: an sqlite3.StructBinder.StructType-type struct. This
       must be a populated (except for the methods) object of type
       sqlite3_io_methods (for the "io" entry) or sqlite3_vfs (for the
       "vfs" entry).

     - `methods`: an object mapping sqlite3_io_methods method names
       (e.g. 'xClose') to JS implementations of those methods. The JS
       implementations must be call-compatible with their native
       counterparts.

     For each of those object, this function passes its (`struct`,
     `methods`, (optional) `applyArgcCheck`) properties to
     installMethods().

     If the `vfs` entry is set then:

     - Its `struct` property's registerVfs() is called. The
       `vfs` entry may optionally have an `asDefault` property, which
       gets passed as the argument to registerVfs().

     - If `struct.$zName` is falsy and the entry has a string-type
       `name` property, `struct.$zName` is set to the C-string form of
       that `name` value before registerVfs() is called. That string
       gets added to the on-dispose state of the struct.

     On success returns this object. Throws on error.
  */
  vfs.installVfs = function(opt){
    let count = 0;
    const propList = ['io','vfs'];
    for(const key of propList){
      const o = opt[key];
      if(o){
        ++count;
        o.struct.installMethods(o.methods, !!o.applyArgcCheck);
        if('vfs'===key){
          if(!o.struct.$zName && 'string'===typeof o.name){
            o.struct.addOnDispose(
              o.struct.$zName = wasm.allocCString(o.name)
            );
          }
          o.struct.registerVfs(!!o.asDefault);
        }
      }
    }
    if(!count) toss("Misuse: installVfs() options object requires at least",
                    "one of:", propList);
    return this;
  };
}/*sqlite3ApiBootstrap.initializers.push()*/);
/*
** 2022-11-30
**
** The author disclaims copyright to this source code.  In place of a
** legal notice, here is a blessing:
**
** *   May you do good and not evil.
** *   May you find forgiveness for yourself and forgive others.
** *   May you share freely, never taking more than you give.
*/

/**
   This file installs sqlite3.vtab, a namespace of helpers for use in
   the creation of JavaScript implementations virtual tables. If built
   without virtual table support then this function does nothing.
*/
'use strict';
globalThis.sqlite3ApiBootstrap.initializers.push(function(sqlite3){
  if( !sqlite3.wasm.exports.sqlite3_declare_vtab ){
    return;
  }
  const wasm = sqlite3.wasm, capi = sqlite3.capi, toss = sqlite3.util.toss3;
  const vtab = Object.create(null);
  sqlite3.vtab = vtab;

  const sii = capi.sqlite3_index_info;
  /**
     If n is >=0 and less than this.$nConstraint, this function
     returns either a WASM pointer to the 0-based nth entry of
     this.$aConstraint (if passed a truthy 2nd argument) or an
     sqlite3_index_info.sqlite3_index_constraint object wrapping that
     address (if passed a falsy value or no 2nd argument). Returns a
     falsy value if n is out of range.
  */
  sii.prototype.nthConstraint = function(n, asPtr=false){
    if(n<0 || n>=this.$nConstraint) return false;
    const ptr = wasm.ptr.add(
      this.$aConstraint,
      sii.sqlite3_index_constraint.structInfo.sizeof * n
    );
    return asPtr ? ptr : new sii.sqlite3_index_constraint(ptr);
  };

  /**
     Works identically to nthConstraint() but returns state from
     this.$aConstraintUsage, so returns an
     sqlite3_index_info.sqlite3_index_constraint_usage instance
     if passed no 2nd argument or a falsy 2nd argument.
  */
  sii.prototype.nthConstraintUsage = function(n, asPtr=false){
    if(n<0 || n>=this.$nConstraint) return false;
    const ptr = wasm.ptr.add(
      this.$aConstraintUsage,
      sii.sqlite3_index_constraint_usage.structInfo.sizeof * n
    );
    return asPtr ? ptr : new sii.sqlite3_index_constraint_usage(ptr);
  };

  /**
     If n is >=0 and less than this.$nOrderBy, this function
     returns either a WASM pointer to the 0-based nth entry of
     this.$aOrderBy (if passed a truthy 2nd argument) or an
     sqlite3_index_info.sqlite3_index_orderby object wrapping that
     address (if passed a falsy value or no 2nd argument). Returns a
     falsy value if n is out of range.
  */
  sii.prototype.nthOrderBy = function(n, asPtr=false){
    if(n<0 || n>=this.$nOrderBy) return false;
    const ptr = wasm.ptr.add(
      this.$aOrderBy,
      sii.sqlite3_index_orderby.structInfo.sizeof * n
    );
    return asPtr ? ptr : new sii.sqlite3_index_orderby(ptr);
  };

  /**
     Internal factory function for xVtab and xCursor impls.
  */
  const __xWrapFactory = function(methodName,StructType){
    return function(ptr,removeMapping=false){
      if(0===arguments.length) ptr = new StructType;
      if(ptr instanceof StructType){
        //T.assert(!this.has(ptr.pointer));
        this.set(ptr.pointer, ptr);
        return ptr;
      }else if(!wasm.isPtr(ptr)){
        sqlite3.SQLite3Error.toss("Invalid argument to",methodName+"()");
      }
      let rc = this.get(ptr);
      if(removeMapping) this.delete(ptr);
      return rc;
    }.bind(new Map);
  };

  /**
     A factory function which implements a simple lifetime manager for
     mappings between C struct pointers and their JS-level wrappers.
     The first argument must be the logical name of the manager
     (e.g. 'xVtab' or 'xCursor'), which is only used for error
     reporting. The second must be the capi.XYZ struct-type value,
     e.g. capi.sqlite3_vtab or capi.sqlite3_vtab_cursor.

     Returns an object with 4 methods: create(), get(), unget(), and
     dispose(), plus a StructType member with the value of the 2nd
     argument. The methods are documented in the body of this
     function.
  */
  const StructPtrMapper = function(name, StructType){
    const __xWrap = __xWrapFactory(name,StructType);
    /**
       This object houses a small API for managing mappings of (`T*`)
       to StructType<T> objects, specifically within the lifetime
       requirements of sqlite3_module methods.
    */
    return Object.assign(Object.create(null),{
      /** The StructType object for this object's API. */
      StructType,
      /**
         Creates a new StructType object, writes its `pointer`
         value to the given output pointer, and returns that
         object. Its intended usage depends on StructType:

         sqlite3_vtab: to be called from sqlite3_module::xConnect()
         or xCreate() implementations.

         sqlite3_vtab_cursor: to be called from xOpen().

         This will throw if allocation of the StructType instance
         fails or if ppOut is not a pointer-type value.
      */
      create: (ppOut)=>{
        const rc = __xWrap();
        wasm.pokePtr(ppOut, rc.pointer);
        return rc;
      },
      /**
         Returns the StructType object previously mapped to the
         given pointer using create(). Its intended usage depends
         on StructType:

         sqlite3_vtab: to be called from sqlite3_module methods which
         take a (sqlite3_vtab*) pointer _except_ for
         xDestroy()/xDisconnect(), in which case unget() or dispose().

         sqlite3_vtab_cursor: to be called from any sqlite3_module methods
         which take a `sqlite3_vtab_cursor*` argument except xClose(),
         in which case use unget() or dispose().

         Rule to remember: _never_ call dispose() on an instance
         returned by this function.
      */
      get: (pCObj)=>__xWrap(pCObj),
      /**
         Identical to get() but also disconnects the mapping between the
         given pointer and the returned StructType object, such that
         future calls to this function or get() with the same pointer
         will return the undefined value. Its intended usage depends
         on StructType:

         sqlite3_vtab: to be called from sqlite3_module::xDisconnect() or
         xDestroy() implementations or in error handling of a failed
         xCreate() or xConnect().

         sqlite3_vtab_cursor: to be called from xClose() or during
         cleanup in a failed xOpen().

         Calling this method obligates the caller to call dispose() on
         the returned object when they're done with it.
      */
      unget: (pCObj)=>__xWrap(pCObj,true),
      /**
         Works like unget() plus it calls dispose() on the
         StructType object.
      */
      dispose: (pCObj)=>__xWrap(pCObj,true)?.dispose?.()
    });
  };

  /**
     A lifetime-management object for mapping `sqlite3_vtab*`
     instances in sqlite3_module methods to capi.sqlite3_vtab
     objects.

     The API docs are in the API-internal StructPtrMapper().
  */
  vtab.xVtab = StructPtrMapper('xVtab', capi.sqlite3_vtab);

  /**
     A lifetime-management object for mapping `sqlite3_vtab_cursor*`
     instances in sqlite3_module methods to capi.sqlite3_vtab_cursor
     objects.

     The API docs are in the API-internal StructPtrMapper().
  */
  vtab.xCursor = StructPtrMapper('xCursor', capi.sqlite3_vtab_cursor);

  /**
     Convenience form of creating an sqlite3_index_info wrapper,
     intended for use in xBestIndex implementations. Note that the
     caller is expected to call dispose() on the returned object
     before returning. Though not _strictly_ required, as that object
     does not own the pIdxInfo memory, it is nonetheless good form.
  */
  vtab.xIndexInfo = (pIdxInfo)=>new capi.sqlite3_index_info(pIdxInfo);

  /**
     Given an sqlite3_module method name and error object, this
     function returns sqlite3.capi.SQLITE_NOMEM if (e instanceof
     sqlite3.WasmAllocError), else it returns its second argument. Its
     intended usage is in the methods of a sqlite3_vfs or
     sqlite3_module:

     ```
     try{
      let rc = ...
      return rc;
     }catch(e){
       return sqlite3.vtab.xError(
                'xColumn', e, sqlite3.capi.SQLITE_XYZ);
       // where SQLITE_XYZ is some call-appropriate result code.
     }
     ```

     If no 3rd argument is provided, its default depends on
     the error type:

     - An sqlite3.WasmAllocError always resolves to capi.SQLITE_NOMEM.

     - If err is an SQLite3Error then its `resultCode` property
       is used.

     - If all else fails, capi.SQLITE_ERROR is used.

     If xError.errorReporter is a function, it is called in
     order to report the error, else the error is not reported.
     If that function throws, that exception is ignored.
  */
  vtab.xError = function f(methodName, err, defaultRc){
    if(f.errorReporter instanceof Function){
      try{f.errorReporter("sqlite3_module::"+methodName+"(): "+err.message);}
      catch(e){/*ignored*/}
    }
    let rc;
    if(err instanceof sqlite3.WasmAllocError) rc = capi.SQLITE_NOMEM;
    else if(arguments.length>2) rc = defaultRc;
    else if(err instanceof sqlite3.SQLite3Error) rc = err.resultCode;
    return rc || capi.SQLITE_ERROR;
  };
  vtab.xError.errorReporter = 1 ? sqlite3.config.error.bind(sqlite3.config) : false;

  /**
     A helper for sqlite3_vtab::xRowid() and xUpdate()
     implementations. It must be passed the final argument to one of
     those methods (an output pointer to an int64 row ID) and the
     value to store at the output pointer's address. Returns the same
     as wasm.poke() and will throw if the 1st or 2nd arguments
     are invalid for that function.

     Example xRowid impl:

     ```
     const xRowid = (pCursor, ppRowid64)=>{
       const c = vtab.xCursor(pCursor);
       vtab.xRowid(ppRowid64, c.myRowId);
       return 0;
     };
     ```
  */
  vtab.xRowid = (ppRowid64, value)=>wasm.poke(ppRowid64, value, 'i64');

  /**
     A helper to initialize and set up an sqlite3_module object for
     later installation into individual databases using
     sqlite3_create_module(). Requires an object with the following
     properties:

     - `methods`: an object containing a mapping of properties with
       the C-side names of the sqlite3_module methods, e.g. xCreate,
       xBestIndex, etc., to JS implementations for those functions.
       Certain special-case handling is performed, as described below.

     - `catchExceptions` (default=false): if truthy, the given methods
       are not mapped as-is, but are instead wrapped inside wrappers
       which translate exceptions into result codes of SQLITE_ERROR or
       SQLITE_NOMEM, depending on whether the exception is an
       sqlite3.WasmAllocError. In the case of the xConnect and xCreate
       methods, the exception handler also sets the output error
       string to the exception's error string.

     - OPTIONAL `struct`: a sqlite3.capi.sqlite3_module() instance. If
       not set, one will be created automatically. If the current
       "this" is-a sqlite3_module then it is unconditionally used in
       place of `struct`.

     - OPTIONAL `iVersion`: if set, it must be an integer value and it
       gets assigned to the `$iVersion` member of the struct object.
       If it's _not_ set, and the passed-in `struct` object's `$iVersion`
       is 0 (the default) then this function attempts to define a value
       for that property based on the list of methods it has.

     If `catchExceptions` is false, it is up to the client to ensure
     that no exceptions escape the methods, as doing so would move
     them through the C API, leading to undefined
     behavior. (vtab.xError() is intended to assist in reporting
     such exceptions.)

     Certain methods may refer to the same implementation. To simplify
     the definition of such methods:

     - If `methods.xConnect` is `true` then the value of
       `methods.xCreate` is used in its place, and vice versa. sqlite
       treats xConnect/xCreate functions specially if they are exactly
       the same function (same pointer value).

     - If `methods.xDisconnect` is true then the value of
       `methods.xDestroy` is used in its place, and vice versa.

     This is to facilitate creation of those methods inline in the
     passed-in object without requiring the client to explicitly get a
     reference to one of them in order to assign it to the other
     one.

     The `catchExceptions`-installed handlers will account for
     identical references to the above functions and will install the
     same wrapper function for both.

     The given methods are expected to return integer values, as
     expected by the C API. If `catchExceptions` is truthy, the return
     value of the wrapped function will be used as-is and will be
     translated to 0 if the function returns a falsy value (e.g. if it
     does not have an explicit return). If `catchExceptions` is _not_
     active, the method implementations must explicitly return integer
     values.

     Throws on error. On success, returns the sqlite3_module object
     (`this` or `opt.struct` or a new sqlite3_module instance,
     depending on how it's called).
  */
  vtab.setupModule = function(opt){
    let createdMod = false;
    const mod = (this instanceof capi.sqlite3_module)
          ? this : (opt.struct || (createdMod = new capi.sqlite3_module()));
    try{
      const methods = opt.methods || toss("Missing 'methods' object.");
      for(const e of Object.entries({
        // -----^ ==> [k,v] triggers a broken code transformation in
        // some versions of the emsdk toolchain.
        xConnect: 'xCreate', xDisconnect: 'xDestroy'
      })){
        // Remap X=true to X=Y for certain X/Y combinations
        const k = e[0], v = e[1];
        if(true === methods[k]) methods[k] = methods[v];
        else if(true === methods[v]) methods[v] = methods[k];
      }
      if(opt.catchExceptions){
        const fwrap = function(methodName, func){
          if(['xConnect','xCreate'].indexOf(methodName) >= 0){
            return function(pDb, pAux, argc, argv, ppVtab, pzErr){
              try{return func(...arguments) || 0}
              catch(e){
                if(!(e instanceof sqlite3.WasmAllocError)){
                  wasm.dealloc(wasm.peekPtr(pzErr));
                  wasm.pokePtr(pzErr, wasm.allocCString(e.message));
                }
                return vtab.xError(methodName, e);
              }
            };
          }else{
            return function(...args){
              try{return func(...args) || 0}
              catch(e){
                return vtab.xError(methodName, e);
              }
            };
          }
        };
        const mnames = [
          'xCreate', 'xConnect', 'xBestIndex', 'xDisconnect',
          'xDestroy', 'xOpen', 'xClose', 'xFilter', 'xNext',
          'xEof', 'xColumn', 'xRowid', 'xUpdate',
          'xBegin', 'xSync', 'xCommit', 'xRollback',
          'xFindFunction', 'xRename', 'xSavepoint', 'xRelease',
          'xRollbackTo', 'xShadowName'
        ];
        const remethods = Object.create(null);
        for(const k of mnames){
          const m = methods[k];
          if(!(m instanceof Function)) continue;
          else if('xConnect'===k && methods.xCreate===m){
            remethods[k] = methods.xCreate;
          }else if('xCreate'===k && methods.xConnect===m){
            remethods[k] = methods.xConnect;
          }else{
            remethods[k] = fwrap(k, m);
          }
        }
        mod.installMethods(remethods, false);
      }else{
        // No automatic exception handling. Trust the client
        // to not throw.
        mod.installMethods(
          methods, !!opt.applyArgcCheck/*undocumented option*/
        );
      }
      if(0===mod.$iVersion){
        let v;
        if('number'===typeof opt.iVersion) v = opt.iVersion;
        else if(mod.$xIntegrity) v = 4;
        else if(mod.$xShadowName) v = 3;
        else if(mod.$xSavePoint || mod.$xRelease || mod.$xRollbackTo) v = 2;
        else v = 1;
        mod.$iVersion = v;
      }
    }catch(e){
      if(createdMod) createdMod.dispose();
      throw e;
    }
    return mod;
  }/*setupModule()*/;

  /**
     Equivalent to calling vtab.setupModule() with this sqlite3_module
     object as the call's `this`.
  */
  capi.sqlite3_module.prototype.setupModule = function(opt){
    return vtab.setupModule.call(this, opt);
  };
}/*sqlite3ApiBootstrap.initializers.push()*/);
/*
  2025-11-21

  The author disclaims copyright to this source code.  In place of a
  legal notice, here is a blessing:

  *   May you do good and not evil.
  *   May you find forgiveness for yourself and forgive others.
  *   May you share freely, never taking more than you give.

  ***********************************************************************

  This file houses the "kvvfs" pieces of the SQLite3 JS API. Most of
  kvvfs is implemented in src/os_kv.c and exposed/extended for use
  here via sqlite3-wasm.c.

  Main project home page: https://sqlite.org

  Documentation home page: https://sqlite.org/wasm
*/

/**
   kvvfs - the Key/Value VFS - is an SQLite3 VFS which delegates
   storage of its pages and metadata to a key-value store.

   It was conceived in order to support JS's localStorage and
   sessionStorage objects. Its native implementation uses files as
   key/value storage (one file per record) but the JS implementation
   replaces a few methods so that it can use the aforementioned
   objects as storage.

   It uses a bespoke ASCII encoding to store each db page as a
   separate record and stores some metadata, like the db's unencoded
   size and its journal, as individual records.

   kvvfs is significantly less efficient than a plain in-memory db but
   it also, as a side effect of its design, offers a JSON-friendly
   interchange format for exporting and importing databases.

   kvvfs is _not_ designed for heavy db loads. It is relatively
   malloc()-heavy, having to de/allocate frequently, and it
   spends much of its time converting the raw db pages into and out of
   an ASCII encoding.

   But it _does_ work and is "performant enough" for db work of the
   scale of a db which will fit within sessionStorage or localStorage
   (just 2-3mb).

   "Version 2" extends it to support using Storage-like objects as
   backing storage, Storage being the JS class which localStorage and
   sessionStorage both derive from. This essentially moves the backing
   store from whatever localStorage and sessionStorage use to an
   in-memory object.

   This effort is primarily a stepping stone towards eliminating, if
   it proves possible, the POSIX I/O API dependencies in SQLite's WASM
   builds. That is: if this VFS works properly, it can be set as the
   default VFS and we can eliminate the "unix" VFS from the JS/WASM
   builds (as opposed to server-wise/WASI builds). That still, as of
   2025-11-23, a ways away, but it's the main driver for version 2 of
   kvvfs.

   Version 2 remains compatible with version 1 databases and always
   writes localStorage/sessionStorage metadata in the v1 format, so
   such dbs can be manipulated freely by either version. For transient
   storage objects (new in version 2), the format of its record keys
   is simpified, requiring less space than v1 keys by eliding
   redundant (in this context) info from the keys.

   Another benefit of v2 is its ability to export dbs into a
   JSON-friendly (but not human-friendly) format.

   A potential, as-yet-unproven, benefit, would be the ability to plug
   arbitrary Storage-compatible objects in so that clients could,
   e.g. asynchronously post updates to db pages to some back-end for
   backups.
*/
globalThis.sqlite3ApiBootstrap.initializers.push(function(sqlite3){
  if( sqlite3.config.disable?.vfs?.kvvfs ){
    return;
  }
  'use strict';
  const capi = sqlite3.capi,
        sqlite3_kvvfs_methods = capi.sqlite3_kvvfs_methods,
        KVVfsFile = capi.KVVfsFile,
        pKvvfs = sqlite3.capi.sqlite3_vfs_find("kvvfs")

  /* These are JS plumbing, not part of the public API */
  delete capi.sqlite3_kvvfs_methods;
  delete capi.KVVfsFile;

  if( !pKvvfs ) return /* nothing to do */;
  if( 0 ){
    /* This working would be our proverbial holy grail, in that it
       would allow us to eliminate the current default VFS, which
       relies on POSIX I/O APIs. Eliminating that dependency would get
       us one giant step closer to creating wasi-sdk builds. */
    capi.sqlite3_vfs_register(pKvvfs, 1);
  }

  const util = sqlite3.util,
        wasm = sqlite3.wasm,
        toss3 = util.toss3,
        hop = (o,k)=>Object.prototype.hasOwnProperty.call(o,k);

  const kvvfsMethods = new sqlite3_kvvfs_methods(
    /* Wraps the static sqlite3_api_methods singleton */
    wasm.exports.sqlite3__wasm_kvvfs_methods()
  );
  util.assert( 32<=kvvfsMethods.$nKeySize, "unexpected kvvfsMethods.$nKeySize: "+kvvfsMethods.$nKeySize);

  /**
     Most of the VFS-internal state.
   */
  const cache = Object.assign(Object.create(null),{
    /** Regex matching journal file names. */
    rxJournalSuffix: /-journal$/,
    /** Frequently-used C-string. */
    zKeyJrnl: wasm.allocCString("jrnl"),
    /** Frequently-used C-string. */
    zKeySz: wasm.allocCString("sz"),
    /**
       The maximum size of a kvvfs record key. It is historically only
       32, a limitation currently retained only because it's convenient to
       do so (the underlying code has outgrown the need for the artifically
       low limit).

       We cache this value here because the end of this init code will
       dispose of kvvfsMethods, invalidating it.
    */
    keySize: kvvfsMethods.$nKeySize,
    /**
       WASM heap memory buffers to optimize out some frequent
       allocations.
    */
    buffer: Object.assign(Object.create(null),{
      /**
         The size of each buffer in this.pool.

         kvvfsMethods.$nBufferSize is slightly larger than the output
         space needed for a kvvfs-encoded 64kb db page in a worse-cast
         encoding (128kb). It is not suitable for arbitrary buffer
         use, only page de/encoding.
      */
      n: kvvfsMethods.$nBufferSize,
      /**
         Map of buffer ids to wasm.alloc()'d pointers of size
         this.n. (Re)used by various internals.

         Buffer ids 0 and 1 are used in the API internals.  Other
         names are used in higher-level APIs.

         See memBuffer() and memBufferFree().
      */
      pool: Object.create(null)
    })
  });

  /**
     Returns a (cached) wasm.alloc()'d buffer of cache.buffer.n size,
     throwing on OOM.

     We leak this one-time alloc because we've no better option.
     sqlite3_vfs does not have a finalizer, so we've no place to hook
     in the cleanup. We "could" extend sqlite3_shutdown() to have a
     cleanup list for stuff like this but that function is never
     used in JS, so it's hardly worth it.
  */
  cache.memBuffer = (id=0)=>cache.buffer.pool[id] ??= wasm.alloc(cache.buffer.n);

  /** Frees the buffer with the given id. */
  cache.memBufferFree = (id)=>{
    const b = cache.buffer.pool[id];
    if( b ){
      wasm.dealloc(b);
      delete cache.buffer.pool[id];
    }
  };

  const noop = ()=>{};
  const debug = sqlite3.__isUnderTest
        ? (...args)=>sqlite3.config.debug?.("kvvfs:", ...args)
        : noop;
  const warn = (...args)=>sqlite3.config.warn?.("kvvfs:", ...args);
  const error = (...args)=>sqlite3.config.error?.("kvvfs:", ...args);

  /**
     Implementation of JS's Storage interface for use as backing store
     of the kvvfs. Storage is a native class and its constructor
     cannot be legally called from JS, making it impossible to
     directly subclass Storage. This class implements (only) the
     Storage interface, to make it a drop-in replacement for
     localStorage/sessionStorage. (Any behavioral discrepancies are to
     be considered bugs.)

     This impl simply proxies a plain, prototype-less Object, suitable
     for JSON-ing.

     Design note: Storage has a bit of an odd iteration-related
     interface as does not (AFAIK) specify specific behavior regarding
     modification during traversal. Because of that, this class does
     some seemingly unnecessary things with its #keys member, deleting
     and recreating it whenever a property index might be invalidated.
  */
  class KVVfsStorage {
    #map = Object.create(null);
    #keys = null;
    #size = 0;

    constructor(){
      this.clear();
    }

    #getKeys(){
      return this.#keys ??= Object.keys(this.#map);
    }

    key(n){
      if(n < 0 || n >= this.#size) return null;
      return this.#getKeys()[n];
    }

    getItem(k){
      return this.#map[k] ?? null;
    }

    setItem(k,v){
      if( !(k in this.#map) ){
        ++this.#size;
        this.#keys = null;
      }
      this.#map[k] = ''+v;
    }

    removeItem(k){
      if( k in this.#map ){
        delete this.#map[k];
        --this.#size;
        this.#keys = null;
      }
    }

    clear(){
      this.#map = Object.create(null);
      this.#keys = null;
      this.#size = 0;
    }

    get length() {
      return this.#size;
    }
  }/*KVVfsStorage*/;

  /** True if v is the name of one of the special persistant Storage
      objects. */
  const kvvfsIsPersistentName = (v)=>'local'===v || 'session'===v;

  /**
     Keys in kvvfs have a prefix of "kvvfs-NAME-", where NAME is the
     db name. This key is redundant in JS but it's how kvvfs works (it
     saves each key to a separate file, so needs a distinct namespace
     per data source name). We retain this prefix in 'local' and
     'session' storage for backwards compatibility and so that they
     can co-exist with client data in their storage, but we elide them
     from "v2" storage, where they're superfluous.
  */
  const kvvfsKeyPrefix = (v)=>kvvfsIsPersistentName(v) ? 'kvvfs-'+v+'-' : '';

  /**
     Throws if storage name n (JS string) is not valid for use as a
     storage name.  Much of this goes back to kvvfs having a fixed
     buffer size for its keys, and the storage name needing to be
     encoded in the keys for local/session storage.

     The second argument must only be true when called from xOpen() -
     it makes names with a "-journal" suffix legal.
  */
  const validateStorageName = function(n,mayBeJournal=false){
    if( kvvfsIsPersistentName(n) ) return;
    const len = (new Blob([n])).size/*byte length*/;
    if( !len ) toss3(capi.SQLITE_MISUSE, "Empty name is not permitted.");
    let maxLen = cache.keySize - 1;
    if( cache.rxJournalSuffix.test(n) ){
      if( !mayBeJournal ){
        toss3(capi.SQLITE_MISUSE,
              "Storage names may not have a '-journal' suffix.");
      }
    }else if( ['-wal','-shm'].filter(v=>n.endsWith(v)).length ){
      toss3(capi.SQLITE_MISUSE,
            "Storage names may not have a -wal or -shm suffix.");
    }else{
      maxLen -= 8 /* so we have space for a matching "-journal" suffix */;
    }
    if( len > maxLen ){
      toss3(capi.SQLITE_RANGE, "Storage name is too long. Limit =", maxLen);
    }
    let i;
    for( i = 0; i < len; ++i ){
      const ch = n.codePointAt(i);
      if( ch<32 ){
        toss3(capi.SQLITE_RANGE,
              "Illegal character ("+ch+"d) in storage name:",n);
      }
    }
  };

  /**
     Create a new instance of the objects which go into
     cache.storagePool, with a refcount of 1. If passed a Storage-like
     object as its second argument, it is used for the storage,
     otherwise it creates a new KVVfsStorage object.
  */
  const newStorageObj = (name,storage=undefined)=>Object.assign(Object.create(null),{
    /**
       JS string value of this KVVfsFile::$zClass. i.e. the storage's
       name.
    */
    jzClass: name,
    /**
       Refcount. This keeps dbs and journals pointing to the same
       storage for the life of both and enables kvvfs to behave more
       like a conventional filesystem (a stepping stone towards
       downstream API goals). Managed by xOpen() and xClose().
    */
    refc: 1,
    /**
       If true, this storage will be removed by xClose() or
       sqlite3_js_kvvfs_unlink() when refc reaches 0. The others will
       persist when refc==0, to give the illusion of real back-end
       storage. Managed by xOpen() and sqlite3_js_kvvfs_reserve(). By
       default this is false but the delete-on-close=1 flag can be
       used to set this to true.
    */
    deleteAtRefc0: false,
    /**
       The backing store. Must implement the Storage interface.
    */
    storage: storage || new KVVfsStorage,
    /**
       The storage prefix used for kvvfs keys.  It is
       "kvvfs-STORAGENAME-" for local/session storage and an empty
       string for other storage. local/session storage must use the
       long form (A) for backwards compatibility and (B) so that kvvfs
       can coexist with non-db client data in those backends.  Neither
       (A) nor (B) are concerns for KVVfsStorage objects.

       This prefix mirrors the one generated by os_kv.c's
       kvrecordMakeKey() and must stay in sync with that one.
    */
    keyPrefix: kvvfsKeyPrefix(name),
    /**
       KVVfsFile instances currently using this storage. Managed by
       xOpen() and xClose().
    */
    files: [],
    /**
       If set, it's an array of objects with various event
       callbacks. See sqlite3_js_kvvfs_listen(). When there are no
       listeners, this member is set to undefined (instead of an empty
       array) to allow us to more easily optimize out calls to
       notifyListeners() for the common case of no listeners.
    */
    listeners: undefined
  });

  /**
     Public interface for kvvfs v2. The capi.sqlite3_js_kvvfs_...()
     routines remain in place for v1. Some members of this class proxy
     to those functions but use different default argument values in
     some cases.
  */
  const kvvfs = sqlite3.kvvfs = Object.create(null);
  if( sqlite3.__isUnderTest ){
    /* For inspection via the dev tools console. */
    kvvfs.log = Object.assign(Object.create(null),{
      xOpen: false,
      xClose: false,
      xWrite: false,
      xRead: false,
      xSync: false,
      xAccess: false,
      xFileControl: false,
      xRcrdRead: false,
      xRcrdWrite: false,
      xRcrdDelete: false,
    });
  }

  /**
     Deletes the cache.storagePool entries for store (a
     cache.storagePool entry) and its db/journal counterpart.
  */
  const deleteStorage = function(store){
    const other = cache.rxJournalSuffix.test(store.jzClass)
          ? store.jzClass.replace(cache.rxJournalSuffix,'')
          : store.jzClass+'-journal';
    kvvfs?.log?.xClose
      && debug("cleaning up storage handles [", store.jzClass, other,"]",store);
    delete cache.storagePool[store.jzClass];
    delete cache.storagePool[other];
    if( !sqlite3.__isUnderTest ){
      /* In test runs, leave these for inspection. If we delete them here,
         any prior dumps of them emitted via the console get cleared out
         because the console shows live objects instead of call-time
         static dumps. */
      delete store.storage;
      delete store.refc;
    }
  };

  /**
     Add both store.jzClass and store.jzClass+"-journal"
     to cache,storagePool.
  */
  const installStorageAndJournal = (store)=>
        cache.storagePool[store.jzClass] =
        cache.storagePool[store.jzClass+'-journal'] = store;

  /**
     The public name of the current thread's transient storage
     object. A storage object with this name gets preinstalled.
  */
  const nameOfThisThreadStorage = '.';

  /**
     Map of JS-stringified KVVfsFile::zClass names to
     reference-counted Storage objects. These objects are created in
     xOpen(). Their refcount is decremented in xClose(), and the
     record is destroyed if the refcount reaches 0. We refcount so
     that concurrent active xOpen()s on a given name, and within a
     given thread, use the same storage object.
  */
  cache.storagePool = Object.assign(Object.create(null),{
    /* Start off with mappings for well-known names. */
    [nameOfThisThreadStorage]: newStorageObj(nameOfThisThreadStorage)
  });

  if( globalThis.Storage ){
    /* If available, install local/session storage. */
    if( globalThis.localStorage instanceof globalThis.Storage ){
      cache.storagePool.local = newStorageObj('local', globalThis.localStorage);
    }
    if( globalThis.sessionStorage instanceof globalThis.Storage ){
      cache.storagePool.session = newStorageObj('session', globalThis.sessionStorage);
    }
  }

  cache.builtinStorageNames = Object.keys(cache.storagePool);

  const isBuiltinName = (n)=>cache.builtinStorageNames.indexOf(n)>-1;

  /* Add "-journal" twins for each cache.storagePool entry... */
  for(const k of Object.keys(cache.storagePool)){
    /* Journals in kvvfs are are stored as individual records within
       their Storage-ish object, named "{storage.keyPrefix}jrnl".  We
       always map the db and its journal to the same Storage
       object. */
    const orig = cache.storagePool[k];
    cache.storagePool[k+'-journal'] = orig;
  }

  cache.setError = (e=undefined, dfltErrCode=capi.SQLITE_ERROR)=>{
    if( e ){
      cache.lastError = e;
      return (e.resultCode | 0) || dfltErrCode;
    }
    delete cache.lastError;
    return 0;
  };

  cache.popError = ()=>{
    const e = cache.lastError;
    delete cache.lastError;
    return e;
  };

  /** Exception handler for notifyListeners(). */
  const catchForNotify = (e)=>{
    warn("kvvfs.listener handler threw:",e);
  };

  const kvvfsDecode = wasm.exports.sqlite3__wasm_kvvfs_decode;
  const kvvfsEncode = wasm.exports.sqlite3__wasm_kvvfs_encode;

  /**
     Listener events and their argument(s) (via the callback(ev)
     ev.data member):

     'open': number of opened handles on this storage.

     'close': number of opened handles on this storage.

     'write': key, value

     'delete': key

     'sync': true if it's from xSync(), false if it's from
     xFileControl().

     For efficiency's sake, all calls to this function should
     be in the form:

     store.listeners && notifyListeners(...);

     Failing to do so will trigger an exceptin in this function (which
     will be ignored but may produce a console warning).
  */
  const notifyListeners = async function(eventName,store,...args){
    try{
      //cache.rxPageNoSuffix ??= /(\d+)$/;
      if( store.keyPrefix && args[0] ){
        args[0] = args[0].replace(store.keyPrefix,'');
      }
      let u8enc, z0, z1, wcache;
      for(const ear of store.listeners){
        const ev = Object.create(null);
        ev.storageName = store.jzClass;
        ev.type = eventName;
        const decodePages = ear.decodePages;
        const f = ear.events[eventName];
        if( f ){
          if( !ear.includeJournal && args[0]==='jrnl' ){
            continue;
          }
          if( 'write'===eventName && ear.decodePages && +args[0]>0 ){
            /* Decode pages to Uint8Array, caching the result in
               wcache in case we have more listeners. */
            ev.data = [args[0]];
            if( wcache?.[args[0]] ){
              ev.data[1] = wcache[args[0]];
              continue;
            }
            u8enc ??= new TextEncoder('utf-8');
            z0 ??= cache.memBuffer(10);
            z1 ??= cache.memBuffer(11);
            const u = u8enc.encode(args[1]);
            const heap = wasm.heap8u();
            heap.set(u, Number(z0));
            heap[wasm.ptr.addn(z0, u.length)] = 0;
            const rc = kvvfsDecode(z0, z1, cache.buffer.n);
            if( rc>0 ){
              wcache ??= Object.create(null);
              wcache[args[0]]
                = ev.data[1]
                = heap.slice(Number(z1), wasm.ptr.addn(z1,rc));
            }else{
              continue;
            }
          }else{
            ev.data = args.length
              ? ((args.length===1) ? args[0] : args)
              : undefined;
          }
          try{f(ev)?.catch?.(catchForNotify)}
          catch(e){
            warn("notifyListeners [",store.jzClass,"]",eventName,e);
          }
        }
      }
    }catch(e){
      catchForNotify(e);
    }
  }/*notifyListeners()*/;

  /**
     Returns the storage object mapped to the given string zClass
     (C-string pointer or JS string).
  */
  const storageForZClass = (zClass)=>
        'string'===typeof zClass
        ? cache.storagePool[zClass]
        : cache.storagePool[wasm.cstrToJs(zClass)];


  const kvvfsMakeKey = wasm.exports.sqlite3__wasm_kvvfsMakeKey;
  /**
     Returns a C string from kvvfsMakeKey() OR returns zKey. In the
     former case the memory is static, so must be copied before a
     second call. zKey MUST be a pointer passed to a VFS/file method,
     to allow us to avoid an alloc and/or an snprintf(). It requires
     C-string arguments for zClass and zKey. zClass may be NULL but
     zKey may not.
  */
  const zKeyForStorage = (store, zClass, zKey)=>{
    //debug("zKeyForStorage(",store, wasm.cstrToJs(zClass), wasm.cstrToJs(zKey));
    return (zClass && store.keyPrefix) ? kvvfsMakeKey(zClass, zKey) : zKey;
  };

  const jsKeyForStorage = (store,zClass,zKey)=>
        wasm.cstrToJs(zKeyForStorage(store, zClass, zKey));

  const storageGetDbSize = (store)=>+store.storage.getItem(store.keyPrefix + "sz");

  /**
     sqlite3_file pointers => objects, each of which has:

     .file = KVVfsFile instance

     .jzClass = JS-string form of f.$zClass

     .storage = Storage object. It is shared between a db and its
     journal.
  */
  const pFileHandles = new Map();

  /**
     Original WASM functions for methods we partially override.
  */
  const originalMethods = {
    vfs: Object.create(null),
    ioDb: Object.create(null),
    ioJrnl: Object.create(null)
  };

  /** Returns the appropriate originalMethods[X] instance for the
      given a KVVfsFile instance. */
  const originalIoMethods = (kvvfsFile)=>
        originalMethods[kvvfsFile.$isJournal ? 'ioJrnl' : 'ioDb'];

  const pVfs = new capi.sqlite3_vfs(kvvfsMethods.$pVfs);
  const pIoDb = new capi.sqlite3_io_methods(kvvfsMethods.$pIoDb);
  const pIoJrnl = new capi.sqlite3_io_methods(kvvfsMethods.$pIoJrnl);
  const recordHandler =
        Object.create(null)/** helper for some vfs
                               routines. Populated later. */;
  const kvvfsInternal = Object.assign(Object.create(null),{
    pFileHandles,
    cache,
    storageForZClass,
    KVVfsStorage,
    /**
       BUG: changing to a page size other than the default,
       then vacuuming, corrupts the db. As a workaround,
       until this is resolved, we forcibly disable
       (pragma page_size=...) changes.
    */
    disablePageSizeChange: true
  });
  if( kvvfs.log ){
    // this is a test build
    kvvfs.internal = kvvfsInternal;
  }

  /**
     Implementations for members of the object referred to by
     sqlite3__wasm_kvvfs_methods(). We swap out some native
     implementations with these so that we can use JS Storage for
     their backing store.
  */
  const methodOverrides = {

    /**
       sqlite3_kvvfs_methods's member methods.  These perform the
       fetching, setting, and removal of storage keys on behalf of
       kvvfs. In the native impl these write each db page to a
       separate file. This impl stores each db page as a single
       record in a Storage object which is mapped to zClass.

       A db's size is stored in a record named kvvfs[-storagename]-sz
       and the journal is stored in kvvfs[-storagename]-jrnl. The
       [-storagename] part is a remnant of the native impl (so that
       it has unique filenames per db) and is only used for
       localStorage and sessionStorage. We elide that part (to save
       space) from other storage objects but retain it on those two
       to avoid invalidating pre-version-2 session/localStorage dbs.

       The interface docs for these methods are in src/os_kv.c's
       kvrecordRead(), kvrecordWrite(), and kvrecordDelete().
    */
    recordHandler: {
      xRcrdRead: (zClass, zKey, zBuf, nBuf)=>{
        try{
          const jzClass = wasm.cstrToJs(zClass);
          const store = storageForZClass(jzClass);
          if( !store ) return -1;
          const jXKey = jsKeyForStorage(store, zClass, zKey);
          kvvfs?.log?.xRcrdRead && warn("xRcrdRead", jzClass, jXKey, nBuf, store );
          const jV = store.storage.getItem(jXKey);
          if(null===jV) return -1;
          const nV = jV.length /* We are relying 100% on v being
                               ** ASCII so that jV.length is equal
                               ** to the C-string's byte length. */;
          if( 0 ){
            debug("xRcrdRead", jXKey, store, jV);
          }
          if(nBuf<=0) return nV;
          else if(1===nBuf){
            wasm.poke(zBuf, 0);
            return nV;
          }
          if( nBuf+1<nV ){
            toss3(capi.SQLITE_RANGE,
                  "xRcrdRead()",jzClass,jXKey,
                  "input buffer is too small: need",
                  nV,"but have",nBuf);
          }
          if( 0 ){
            debug("xRcrdRead", nBuf, zClass, wasm.cstrToJs(zClass),
                  wasm.cstrToJs(zKey), nV, jV, store);
          }
          const zV = cache.memBuffer(0);
          //if( !zV ) return -3 /*OOM*/;
          const heap = wasm.heap8();
          let i;
          for(i = 0; i < nV; ++i){
            heap[wasm.ptr.add(zV,i)] = jV.codePointAt(i) & 0xFF;
          }
          heap.copyWithin(
            Number(zBuf), Number(zV), wasm.ptr.addn(zV, i)
          );
          heap[wasm.ptr.add(zBuf, nV)] = 0;
          return nBuf;
        }catch(e){
          error("kvrecordRead()",e);
          cache.setError(e);
          return -2;
        }
      },

      xRcrdWrite: (zClass, zKey, zData)=>{
        try {
          const store = storageForZClass(zClass);
          const jxKey = jsKeyForStorage(store, zClass, zKey);
          const jData = wasm.cstrToJs(zData);
          kvvfs?.log?.xRcrdWrite && warn("xRcrdWrite",jxKey, store);
          store.storage.setItem(jxKey, jData);
          store.listeners && notifyListeners('write', store, jxKey, jData);
          return 0;
        }catch(e){
          error("kvrecordWrite()",e);
          return cache.setError(e, capi.SQLITE_IOERR);
        }
      },

      xRcrdDelete: (zClass, zKey)=>{
        try {
          const store = storageForZClass(zClass);
          const jxKey = jsKeyForStorage(store, zClass, zKey);
          kvvfs?.log?.xRcrdDelete && warn("xRcrdDelete",jxKey, store);
          store.storage.removeItem(jxKey);
          store.listeners && notifyListeners('delete', store, jxKey);
          return 0;
        }catch(e){
          error("kvrecordDelete()",e);
          return cache.setError(e, capi.SQLITE_IOERR);
        }
      }
    }/*recordHandler*/,

    /**
       Override certain operations of the underlying sqlite3_vfs and
       the two sqlite3_io_methods instances so that we can tie
       Storage objects to db names.
    */
    vfs:{
      /* sqlite3_kvvfs_methods::pVfs's methods */
      xOpen: function(pProtoVfs,zName,pProtoFile,flags,pOutFlags){
        cache.popError();
        let zToFree /* alloc()'d memory for temp db name */;
        try{
          if( !zName ){
            zToFree = wasm.allocCString(""+pProtoFile+"."
                                        +(Math.random() * 100000 | 0));
            zName = zToFree;
          }
          const jzClass = wasm.cstrToJs(zName);
          kvvfs?.log?.xOpen && debug("xOpen",jzClass,"flags =",flags);
          validateStorageName(jzClass, true);
          if( (flags & (capi.SQLITE_OPEN_MAIN_DB
                        | capi.SQLITE_OPEN_TEMP_DB
                        | capi.SQLITE_OPEN_TRANSIENT_DB))
              && cache.rxJournalSuffix.test(jzClass) ){
            toss3(capi.SQLITE_ERROR,
                  "DB files may not have a '-journal' suffix.");
          }
          let s = storageForZClass(jzClass);
          if( !s && !(flags & capi.SQLITE_OPEN_CREATE) ){
            toss3(capi.SQLITE_ERROR, "Storage not found:", jzClass);
          }
          const rc = originalMethods.vfs.xOpen(pProtoVfs, zName, pProtoFile,
                                               flags, pOutFlags);
          if( rc ) return rc;
          let deleteAt0 = !!(capi.SQLITE_OPEN_DELETEONCLOSE & flags);
          if(wasm.isPtr(arguments[1]/*original zName*/)){
            if(capi.sqlite3_uri_boolean(zName, "delete-on-close", 0)){
              deleteAt0 = true;
            }
          }
          const f = new KVVfsFile(pProtoFile);
          util.assert(f.$zClass, "Missing f.$zClass");
          f.addOnDispose(zToFree);
          zToFree = undefined;
          //debug("xOpen", jzClass, s);
          if( s ){
            ++s.refc;
            //no if( true===deleteAt0 ) s.deleteAtRefc0 = true;
            s.files.push(f);
            wasm.poke32(pOutFlags, flags);
          }else{
            wasm.poke32(pOutFlags, flags | capi.SQLITE_OPEN_CREATE);
            util.assert( !f.$isJournal, "Opening a journal before its db? "+jzClass );
            /* Map both zName and zName-journal to the same storage. */
            const nm = jzClass.replace(cache.rxJournalSuffix,'');
            s = newStorageObj(nm);
            installStorageAndJournal(s);
            s.files.push(f);
            s.deleteAtRefc0 = deleteAt0;
            kvvfs?.log?.xOpen
              && debug("xOpen installed storage handle [",nm, nm+"-journal","]", s);
          }
          pFileHandles.set(pProtoFile, {store: s, file: f, jzClass});
          s.listeners && notifyListeners('open', s, s.files.length);
          return 0;
        }catch(e){
          warn("xOpen:",e);
          return cache.setError(e);
        }finally{
          zToFree && wasm.dealloc(zToFree);
        }
      }/*xOpen()*/,

      xDelete: function(pVfs, zName, iSyncFlag){
        cache.popError();
        try{
          const jzName = wasm.cstrToJs(zName);
          if( cache.rxJournalSuffix.test(jzName) ){
            recordHandler.xRcrdDelete(zName, cache.zKeyJrnl);
          }/*
             else: historically not done, but maybe otherwise delete
             all db pages from storageForZClass(zName)?
           */
          return 0;
        }catch(e){
          warn("xDelete",e);
          return cache.setError(e);
        }
      },

      xAccess: function(pProtoVfs, zPath, flags, pResOut){
        cache.popError();
        try{
          const s = storageForZClass(zPath);
          const jzPath = s?.jzClass || wasm.cstrToJs(zPath);
          if( kvvfs?.log?.xAccess ){
            debug("xAccess",jzPath,"flags =",
                  flags,"*pResOut =",wasm.peek32(pResOut),
                  "store =",s);
          }
          if( !s ){
            // From the API docs:
            /** The xAccess method returns [SQLITE_OK] on success or some
             ** non-zero error code if there is an I/O error or if the name of
             ** the file given in the second argument is illegal.
             */
            // However, returning non-0 from here is fatal, so we don't do that.
            try{validateStorageName(jzPath)}
            catch(e){
              //warn("xAccess is ignoring name validation failure:",e);
              wasm.poke32(pResOut, 0);
              return 0;
            }
          }
          if( s ){
            const key = s.keyPrefix+
                  (cache.rxJournalSuffix.test(jzPath) ? "jrnl" : "1");
            const res = s.storage.getItem(key) ? 0 : 1;
            /* This res value looks completely backwards to me, and
               is the opposite of the native kvvfs's impl, but it's
               working, whereas reimplementing the native one
               faithfully does not. Read the lib-level code of where
               this is invoked, my expectation is that we set res to 0
               for not-exists. */
            //warn("access res",jzPath,res);
            wasm.poke32(pResOut, res);
          }else{
            wasm.poke32(pResOut, 0);
          }
          return 0;
        }catch(e){
          error('xAccess',e);
          return cache.setError(e);
        }
      },

      xRandomness: function(pVfs, nOut, pOut){
        const heap = wasm.heap8u();
        let i = 0;
        const npOut = Number(pOut);
        for(; i < nOut; ++i) heap[npOut + i] = (Math.random()*255000) & 0xFF;
        return nOut;
      },

      xGetLastError: function(pVfs,nOut,pOut){
        const e = cache.popError();
        debug('xGetLastError',e);
        if(e){
          const scope = wasm.scopedAllocPush();
          try{
            const [cMsg, n] = wasm.scopedAllocCString(e.message, true);
            wasm.cstrncpy(pOut, cMsg, nOut);
            if(n > nOut) wasm.poke8(wasm.ptr.add(pOut,nOut,-1), 0);
            debug("set xGetLastError",e.message);
            return (e.resultCode | 0) || capi.SQLITE_IOERR;
          }catch(e){
            return capi.SQLITE_NOMEM;
          }finally{
            wasm.scopedAllocPop(scope);
          }
        }
        return 0;
      }

    }/*.vfs*/,

    /**
       kvvfs has separate sqlite3_api_methods impls for some of the
       methods depending on whether it's a db or journal file. Some
       of the methods use shared impls but others are specific to
       either db or journal files.
    */
    ioDb:{
      /* sqlite3_kvvfs_methods::pIoDb's methods */
      xClose: function(pFile){
        cache.popError();
        try{
          const h = pFileHandles.get(pFile);
          kvvfs?.log?.xClose && debug("xClose", pFile, h);
          if( h ){
            pFileHandles.delete(pFile);
            const s = h.store;//storageForZClass(h.jzClass);
            s.files = s.files.filter((v)=>v!==h.file);
            if( --s.refc<=0 && s.deleteAtRefc0 ){
              deleteStorage(s);
            }
            originalMethods.ioDb/*same for journals*/.xClose(pFile);
            h.file.dispose();
            s.listeners && notifyListeners('close', s, s.files.length);
          }else{
            /* Can happen if xOpen fails */
          }
          return 0;
        }catch(e){
          error("xClose",e);
          return cache.setError(e);
        }
      },

      xFileControl: function(pFile, opId, pArg){
        cache.popError();
        try{
          const h = pFileHandles.get(pFile);
          util.assert(h, "Missing KVVfsFile handle");
          kvvfs?.log?.xFileControl && debug("xFileControl",h,'op =',opId);
          if( opId===capi.SQLITE_FCNTL_PRAGMA
              && kvvfsInternal.disablePageSizeChange ){
            /* pArg== length-3 (char**) */
            //const argv = wasm.cArgvToJs(3, pArg); // the easy way
            const zName = wasm.peekPtr(wasm.ptr.add(pArg, wasm.ptr.size));
            if( "page_size"===wasm.cstrToJs(zName) ){
              kvvfs?.log?.xFileControl
                && debug("xFileControl pragma",wasm.cstrToJs(zName));
              const zVal = wasm.peekPtr(wasm.ptr.add(pArg, 2*wasm.ptr.size));
              if( zVal ){
                /* Without this, pragma page_size=N; followed by a
                   vacuum breaks the db. With this, it continues
                   working but does not actually change the page
                   size. */
                kvvfs?.log?.xFileControl
                  && warn("xFileControl pragma", h,
                          "NOT setting page size to", wasm.cstrToJs(zVal));
                h.file.$szPage = -1;
                return 0/*corrupts: capi.SQLITE_NOTFOUND*/;
              }else if( h.file.$szPage>0 ){
                kvvfs?.log?.xFileControl &&
                  warn("xFileControl", h, "getting page size",h.file.$szPage);
                wasm.pokePtr(pArg, wasm.allocCString(""+h.file.$szPage)
                             /* memory now owned by the library */);
                return 0;//capi.SQLITE_NOTFOUND;
              }
            }
          }
          const rc = originalMethods.ioDb.xFileControl(pFile, opId, pArg);
          if( 0==rc && capi.SQLITE_FCNTL_SYNC===opId ){
            h.store.listeners && notifyListeners('sync', h.store, false);
          }
          return rc;
        }catch(e){
          error("xFileControl",e);
          return cache.setError(e);
        }
      },

      xSync: function(pFile,flags){
        cache.popError();
        try{
          const h = pFileHandles.get(pFile);
          kvvfs?.log?.xSync && debug("xSync", h);
          util.assert(h, "Missing KVVfsFile handle");
          const rc = originalMethods.ioDb.xSync(pFile, flags);
          if( 0==rc && h.store.listeners ) notifyListeners('sync', h.store, true);
          return rc;
        }catch(e){
          error("xSync",e);
          return cache.setError(e);
        }
      },


    }/*.ioDb*/,

    ioJrnl:{
      /* sqlite3_kvvfs_methods::pIoJrnl's methods. Those set to true
         are copied as-is from the ioDb objects. Others are specific
         to journal files. */
      xClose: true,
    }/*.ioJrnl*/
  }/*methodOverrides*/;


  try {
    util.assert( cache.buffer.n>1024*129, "Heap buffer is not large enough"
                 /* Native is SQLITE_KVOS_SZ is 133073 as of this writing */ );
    for(const e of Object.entries(methodOverrides.recordHandler)){
      // Overwrite kvvfsMethods's callbacks
      const k = e[0], f = e[1];
      recordHandler[k] = f;
      if( 0 ){
        // bug: this should work
        kvvfsMethods.installMethod(k, f);
      }else{
        kvvfsMethods[kvvfsMethods.memberKey(k)] =
          wasm.installFunction(kvvfsMethods.memberSignature(k), f);
      }
    }
    for(const e of Object.entries(methodOverrides.vfs)){
      // Overwrite some pVfs entries and stash the original impls
      const k = e[0], f = e[1], km = pVfs.memberKey(k),
            member = pVfs.structInfo.members[k]
            || util.toss("Missing pVfs.structInfo[",k,"]");
      originalMethods.vfs[k] = wasm.functionEntry(pVfs[km]);
      pVfs[km] = wasm.installFunction(member.signature, f);
    }
    for(const e of Object.entries(methodOverrides.ioDb)){
      // Similar treatment for pVfs.$pIoDb a.k.a. pIoDb...
      const k = e[0], f = e[1], km = pIoDb.memberKey(k);
      originalMethods.ioDb[k] = wasm.functionEntry(pIoDb[km])
        || util.toss("Missing native pIoDb[",km,"]");
      pIoDb[km] = wasm.installFunction(pIoDb.memberSignature(k), f);
    }
    for(const e of Object.entries(methodOverrides.ioJrnl)){
      // Similar treatment for pVfs.$pIoJrnl a.k.a. pIoJrnl...
      const k = e[0], f = e[1], km = pIoJrnl.memberKey(k);
      originalMethods.ioJrnl[k] = wasm.functionEntry(pIoJrnl[km])
        || util.toss("Missing native pIoJrnl[",km,"]");
      if( true===f ){
        /* use pIoDb's copy */
        pIoJrnl[km] = pIoDb[km] || util.toss("Missing copied pIoDb[",km,"]");
      }else{
        pIoJrnl[km] = wasm.installFunction(pIoJrnl.memberSignature(k), f);
      }
    }
  }finally{
    kvvfsMethods.dispose();
    pVfs.dispose();
    pIoDb.dispose();
    pIoJrnl.dispose();
  }

  /*
    That gets all of the low-level bits out of the way. What follows
    are the public API additions.
  */

  /**
     Clears all storage used by the kvvfs DB backend, deleting any
     DB(s) stored there.

     Its argument must be the name of a kvvfs storage object:

     - 'session'
     - 'local'
     - '' - see below.
     - A transient kvvfs storage object name.

     In the first two cases, only sessionStorage resp. localStorage is
     cleared. An empty string resolves to both 'local' and 'session'
     storage.

     Returns the number of entries cleared.

     As of kvvfs version 2:

     This API is available in Worker threads but does not have access
     to localStorage or sessionStorage in them. Prior versions did not
     include this API in Worker threads.

     Differences in this function in version 2:

     - It accepts an arbitrary storage name. In v1 this was a silent
     no-op for any names other than ('local','session','').

     - It throws if a db currently has the storage opened UNLESS the
     storage object is localStorage or sessionStorage. That version 1
     did not throw for this case was due to an architectural
     limitation which has since been overcome, but removal of
     JsStorageDb.prototype.clearStorage() would be a backwards compatibility
     break, so this function permits wiping the storage for those two
     cases even if they are opened. Use with care.
  */
  const sqlite3_js_kvvfs_clear = function callee(which){
    if( ''===which ){
      return callee('local') + callee('session');
    }
    const store = storageForZClass(which);
    if( !store ) return 0;
    if( store.files.length ){
      if( globalThis.localStorage===store.storage
          || globalThis.sessionStorage===store.storage ){
        /* backwards compatibility: allow these to be cleared
           while opened. */
      }else{
        /* Interestingly, kvvfs recovers just fine when the storage is
           wiped, so long as the db is not in use and its schema is
           recreated before it's used, but client apps should not have
           to be faced with that eventuality mid-query (where it
           _will_ cause failures). Therefore we disallow it when
           storage handles are opened. Kvvfs version 1 could not
           detect this case - see the if() block above.
        */
        toss3(capi.SQLITE_ACCESS,
              "Cannot clear in-use database storage.");
      }
    }
    const s = store.storage;
    const toRm = [] /* keys to remove */;
    let i, n = s.length;
    //debug("kvvfs_clear",store,s);
    for( i = 0; i < n; ++i ){
      const k = s.key(i);
      //debug("kvvfs_clear ?",k);
      if(!store.keyPrefix || k.startsWith(store.keyPrefix)) toRm.push(k);
    }
    toRm.forEach((kk)=>s.removeItem(kk));
    //alertFilesToReload(store);
    return toRm.length;
  };

  /**
     This routine estimates the approximate amount of
     storage used by the given kvvfs back-end.

     Its arguments are as documented for sqlite3_js_kvvfs_clear(),
     only the operation this performs is different.

     The returned value is twice the "length" value of every matching
     key and value, noting that JavaScript stores each character in 2
     bytes.

     The returned size is not authoritative from the perspective of
     how much data can fit into localStorage and sessionStorage, as
     the precise algorithms for determining those limits are
     unspecified and may include per-entry overhead invisible to
     clients.
  */
  const sqlite3_js_kvvfs_size = function callee(which){
    if( ''===which ){
      return callee('local') + callee('session');
    }
    const store = storageForZClass(which);
    if( !store ) return 0;
    const s = store.storage;
    let i, sz = 0;
    for(i = 0; i < s.length; ++i){
      const k = s.key(i);
      if(!store.keyPrefix || k.startsWith(store.keyPrefix)){
        sz += k.length;
        sz += s.getItem(k).length;
      }
    }
    return sz * 2 /* because JS uses 2-byte char encoding */;
  };

  /**
     Exports a kvvfs storage object to an object, optionally
     JSON-friendly.

     Usages:

     thisfunc(storageName);
     thisfunc(options);

     In the latter case, the options object must be an object with
     the following properties:

     - "name" (string) required. The storage to export.

     - "decodePages" (bool=false). If true, the .pages result property
     holdes Uint8Array objects holding the raw binary-format db
     pages. The default is to use kvvfs-encoded string pages
     (JSON-friendly).

     - "includeJournal" (bool=false). If true and the db has a current
     journal, it is exported as well. (Kvvfs journals are stored as a
     single record within the db's storage object.)

     The returned object is structured as follows...

     - "name": the name of the storage. This is 'local' or 'session'
     for localStorage resp. sessionStorage, and an arbitrary name for
     transient storage. This propery may be changed before passing
     this object to sqlite3_js_kvvfs_import() in order to
     import into a different storage object.

     - "timestamp": the time this function was called, in Unix
     epoch milliseconds.

     - "size": the unencoded db size.

     - "journal": if options.includeJournal is true and this db has a
     journal, it is stored as a string here, otherwise this property
     is not set.

     - "pages": An array holding the raw encoded db pages in their
     proper order.

     Throws if this db is not opened.

     The encoding of the underlying database is not part of this
     interface - it is simply passed on as-is. Interested parties are
     directed to src/os_kv.c in the SQLite source tree, with the
     caveat that that code also does not offer a public interface.
     i.e. the encoding is a private implementation detail of kvvfs.
     The format may be changed in the future but kvvfs will continue
     to support the current form.

     Added in version 3.52.0.
  */
  const sqlite3_js_kvvfs_export = function callee(...args){
    let opt;
    if( 1===args.length && 'object'===typeof args[0] ){
      opt = args[0];
    }else if(args.length){
      opt = Object.assign(Object.create(null),{
        name: args[0],
        //decodePages: true
      });
    }
    const store = opt ? storageForZClass(opt.name) : null;
    if( !store ){
      toss3(capi.SQLITE_NOTFOUND,
            "There is no kvvfs storage named",opt?.name);
    }
    //debug("store to export=",store);
    const s = store.storage;
    const rc = Object.assign(Object.create(null),{
      name: store.jzClass,
      timestamp: Date.now(),
      pages: []
    });
    const pages = Object.create(null);
    let xpages;
    const keyPrefix = store.keyPrefix;
    const rxTail = keyPrefix
          ? /^kvvfs-[^-]+-(\w+)/ /* X... part of kvvfs-NAME-X... */
          : undefined;
    let i = 0, n = s.length;
    for( ; i < n; ++i ){
      const k = s.key(i);
      if( !keyPrefix || k.startsWith(keyPrefix) ){
        let kk = (keyPrefix ? rxTail.exec(k) : undefined)?.[1] ?? k;
        switch( kk ){
          case 'jrnl':
            if( opt.includeJournal ) rc.journal = s.getItem(k);
            break;
          case 'sz':
            rc.size = +s.getItem(k);
            break;
          default:
            kk = +kk /* coerce to number */;
            if( !util.isInt32(kk) || kk<=0 ){
              toss3(capi.SQLITE_RANGE, "Malformed kvvfs key: "+k);
            }
            if( opt.decodePages ){
              const spg = s.getItem(k),
                    n = spg.length,
                    z = cache.memBuffer(0),
                    zDec = cache.memBuffer(1),
                    heap = wasm.heap8u()/* MUST be inited last*/;
              let i = 0;
              for( ; i < n; ++i ){
                heap[wasm.ptr.add(z, i)] = spg.codePointAt(i) & 0xff;
              }
              heap[wasm.ptr.add(z, i)] = 0;
              //debug("Decoding",i,"page bytes");
              const nDec = kvvfsDecode(
                z, zDec, cache.buffer.n
              );
              //debug("Decoded",nDec,"page bytes");
              pages[kk] = heap.slice(Number(zDec), wasm.ptr.addn(zDec, nDec));
            }else{
              pages[kk] = s.getItem(k);
            }
            break;
        }
      }
    }
    if( opt.decodePages ) cache.memBufferFree(1);
    /* Now sort the page numbers and move them into an array. In JS
       property keys are always strings, so we have to coerce them to
       numbers so we can get them sorted properly for the array. */
    Object.keys(pages).map((v)=>+v).sort().forEach(
      (v)=>rc.pages.push(pages[v])
    );
    return rc;
  }/* sqlite3_js_kvvfs_export */;

  /**
     The counterpart of sqlite3_js_kvvfs_export(). Its
     argument must be the result of that function() or
     a compatible one.

     This either replaces the contents of an existing transient
     storage object or installs one named exp.name, setting
     the storage's db contents to that of the exp object.

     Throws on error. Error conditions include:

     - The given storage object is currently opened by any db.
     Performing this page-by-page import would invoke undefined
     behavior on them.

     - Malformed input object.

     If it throws after starting the import then it clears the storage
     before returning, to avoid leaving the db in an undefined
     state. It may throw for any of the above-listed conditions before
     reaching that step, in which case the db is not modified. If
     exp.name refers to a new storage name then if it throws, the name
     does not get installed.

     Added in version 3.52.0.
  */
  const sqlite3_js_kvvfs_import = function(exp, overwrite=false){
    if( !exp?.timestamp
        || !exp.name
        || undefined===exp.size
        || !Array.isArray(exp.pages) ){
      toss3(capi.SQLITE_MISUSE, "Malformed export object.");
    }else if( !exp.size
              || (exp.size !== (exp.size | 0))
              //|| (exp.size % cache.fixedPageSize)
              || exp.size>=0x7fffffff ){
      toss3(capi.SQLITE_RANGE, "Invalid db size: "+exp.size);
    }

    validateStorageName(exp.name);
    let store = storageForZClass(exp.name);
    const isNew = !store;
    if( store ){
      if( !overwrite ){
        //warn("Storage exists:",arguments,store);
        toss3(capi.SQLITE_ACCESS,
              "Storage '"+exp.name+"' already exists and",
              "overwrite was not specified.");
      }else if( !store.files || !store.jzClass ){
        toss3(capi.SQLITE_ERROR,
              "Internal storage object", exp.name,"seems to be malformed.");
      }else if( store.files.length ){
        toss3(capi.SQLITE_IOERR_ACCESS,
              "Cannot import db storage while it is in use.");
      }
      sqlite3_js_kvvfs_clear(exp.name);
    }else{
      store = newStorageObj(exp.name);
      //warn("Installing new storage:",store);
    }
    //debug("Importing store",store.poolEntry.files.length, store);
    //debug("object to import:",exp);
    const keyPrefix = kvvfsKeyPrefix(exp.name);
    let zEnc;
    try{
      /* Force the native KVVfsFile instances to re-read the db
         and page size. */;
      const s = store.storage;
      s.setItem(keyPrefix+'sz', exp.size);
      if( exp.journal ) s.setItem(keyPrefix+'jrnl', exp.journal);
      if( exp.pages[0] instanceof Uint8Array ){
        /* raw binary pages */
        //debug("pages",exp.pages);
        exp.pages.forEach((u,ndx)=>{
          const n = u.length;
          if( 0 && cache.fixedPageSize !== n ){
            util.toss3(capi.SQLITE_RANGE,"Unexpected page size:", n);
          }
          zEnc ??= cache.memBuffer(1);
          const zBin = cache.memBuffer(0),
                heap = wasm.heap8u()/*MUST be inited last*/;
          /* Copy u to the heap and encode the heap copy via C. This
             is _presumably_ faster than porting the encoding algo to
             JS. */
          heap.set(u, Number(zBin));
          heap[wasm.ptr.addn(zBin,n)] = 0;
          const rc = kvvfsEncode(zBin, n, zEnc);
          util.assert( rc < cache.buffer.n,
                       "Impossibly long output - possibly smashed the heap" );
          util.assert( 0===wasm.peek8(wasm.ptr.add(zEnc,rc)),
                       "Expecting NUL-terminated encoded output" );
          const jenc = wasm.cstrToJs(zEnc);
          //debug("(un)encoded page:",u,jenc);
          s.setItem(keyPrefix+(ndx+1), jenc);
        });
      }else if( exp.pages[0] ){
        /* kvvfs-encoded pages */
        exp.pages.forEach((v,ndx)=>s.setItem(keyPrefix+(ndx+1), v));
      }
      if( isNew ) installStorageAndJournal(store);
    }catch{
      if( !isNew ){
        try{sqlite3_js_kvvfs_clear(exp.name);}catch(ee){/*ignored*/}
      }
    }finally{
      if( zEnc ) cache.memBufferFree(1);
    }
    return this;
  };

  /**
     If no kvvfs storage exists with the given name, one is
     installed. If one exists, its reference count is increased so
     that it won't be freed by the closing of a database or journal
     file.

     Throws if the name is not valid for a new storage object.

     Added in version 3.52.0.
  */
  const sqlite3_js_kvvfs_reserve = function(name){
    let store = storageForZClass(name);
    if( store ){
      ++store.refc;
      return;
    }
    validateStorageName(name);
    installStorageAndJournal(newStorageObj(name));
  };

  /**
     Conditionally "unlinks" a kvvfs storage object, reducing its
     reference count by 1.

     This is a no-op if name ends in "-journal" or refers to a
     built-in storage object.

     It will not lower the refcount below the number of
     currently-opened db/journal files for the storage (so that it
     cannot delete it out from under them).

     If the refcount reaches 0 then the storage object is
     removed.

     Returns true if it reduces the refcount, else false.  A result of
     true does not necessarily mean that the storage unit was removed,
     just that its refcount was lowered. Similarly, a result of false
     does not mean that the storage is removed - it may still have
     opened handles.

     Added in version 3.52.0.
  */
  const sqlite3_js_kvvfs_unlink = function(name){
    const store = storageForZClass(name);
    if( !store
        || kvvfsIsPersistentName(store.jzClass)
        || isBuiltinName(store.jzClass)
        || cache.rxJournalSuffix.test(name) ) return false;
    if( store.refc > store.files.length || 0===store.files.length ){
      if( --store.refc<=0 ){
        /* Ignoring deleteAtRefc0 for an explicit unlink */
        deleteStorage(store);
      }
      return true;
    }
    return false;
  };

  /**
     Adds an event listener to a kvvfs storage object. The idea is
     that this can be used to asynchronously back up one kvvfs storage
     object to another or another channel entirely. (The caveat in the
     latter case is that kvvfs's format is not readily consumable by
     downstream code.)

     Its argument must be an object with the following properties:

     - storage: the name of the kvvfs storage object.

     - reserve [=false]: if true, sqlite3_js_kvvfs_reserve() is used
     to ensure that the storage exists if it does not already.
     If this is false and the storage does not exist then an
     exception is thrown.

     - events: an object which may have any of the following
     callback function properties: open, close, write, delete.

     - decodePages [=false]: if true, write events will receive each
     db page write in the form of a Uint8Array holding the raw binary
     db page. The default is to emit the kvvfs-format page because it
     requires no extra work, we already have it in hand, and it's
     often smaller. It's not great for interchange, though.

     - includeJournal [=false]: if true, writes and deletes of
     "jrnl" records are included. If false, no events are sent
     for journal updates.

     Passing the same object to sqlite3_js_kvvfs_unlisten() will
     remove the listener.

     Each one of the events callbacks will be called asynchronously
     when the given storage performs those operations. They may be
     asynchronous functions but are not required to be (the events are
     fired async either way, but making the event callbacks async may
     be advantageous when multiple listeners are involved). All
     exceptions, including those via Promises, are ignored but may (or
     may not) trigger warning output on the console.

     Each callback gets passed a single object with the following
     properties:

     .type = the same as the name of the callback

     .storageName = the name of the storage object

     .data = callback-dependent:

     - 'open' and 'close' get an integer, the number of
     currently-opened handles on the storage.

     - 'write' gets a length-two array holding the key and value which
     were written. The key is always a string, even if it's a db page
     number. For db-page records, the value's type depends on
     opt.decodePages.  All others, including the journal, are strings.
     (The journal, being a kvvfs-specific format, is delivered in
     that same JSON-friendly format.) More details below.

     - 'delete' gets the string-type key of the deleted record.

     - 'sync' gets a boolean value: true if it was triggered by db
     file's xSync(), false if it was triggered by xFileControl().  The
     latter triggers before the xSync() and also triggers if the DB
     has PRAGMA SYNCHRONOUS=OFF (in which case xSync() is not
     triggered).

     The key/value arguments to 'write', and key argument to 'delete',
     are in one of the following forms:

     - 'sz' = the unencoded db size as a string. This specific key is
     key is never deleted, so is only ever passed to 'write' events.

     - 'jrnl' = the current db journal as a kvvfs-encoded string. This
     journal format is not useful anywhere except in the kvvfs
     internals. These events are not fired if opt.includeJournal is
     false.

     - '[1-9][0-9]*' (a db page number) = Its type depends on
     opt.decodePages. These may be written and deleted in arbitrary
     order.

     Design note: JS has StorageEvents but only in the main thread,
     which is why the listeners are not based on that.

     Added in version 3.52.0.
  */
  const sqlite3_js_kvvfs_listen = function(opt){
    if( !opt || 'object'!==typeof opt ){
      toss3(capi.SQLITE_MISUSE, "Expecting a listener object.");
    }
    let store = storageForZClass(opt.storage);
    if( !store ){
      if( opt.storage && opt.reserve ){
        sqlite3_js_kvvfs_reserve(opt.storage);
        store = storageForZClass(opt.storage);
        util.assert(store,
                    "Unexpectedly cannot fetch reserved storage "
                    +opt.storage);
      }else{
        toss3(capi.SQLITE_NOTFOUND,"No such storage:",opt.storage);
      }
    }
    if( opt.events ){
      (store.listeners ??= []).push(opt);
    }
  };

  /**
     Removes the kvvfs event listeners for the given options
     object. It must be passed the same object instance which was
     passed to sqlite3_js_kvvfs_listen().

     This has no side effects if opt is invalid or is not a match for
     any listeners.

     Return true if it unregisters its argument, else false.

     Added in version 3.52.0.
  */
  const sqlite3_js_kvvfs_unlisten = function(opt){
    const store = storageForZClass(opt?.storage);
    if( store?.listeners && opt.events ){
      const n = store.listeners.length;
      store.listeners = store.listeners.filter((v)=>v!==opt);
      const rc = n>store.listeners.length;
      if( !store.listeners.length ){
        // to speed up downstream checks for listeners
        store.listeners = undefined;
      }
      return rc;
    }
    return false;
  };

  sqlite3.kvvfs.reserve =  sqlite3_js_kvvfs_reserve;
  sqlite3.kvvfs.import =   sqlite3_js_kvvfs_import;
  sqlite3.kvvfs.export =   sqlite3_js_kvvfs_export;
  sqlite3.kvvfs.unlink =   sqlite3_js_kvvfs_unlink;
  sqlite3.kvvfs.listen =   sqlite3_js_kvvfs_listen;
  sqlite3.kvvfs.unlisten = sqlite3_js_kvvfs_unlisten;
  sqlite3.kvvfs.exists =   (name)=>!!storageForZClass(name);
  sqlite3.kvvfs.estimateSize = sqlite3_js_kvvfs_size;
  sqlite3.kvvfs.clear =    sqlite3_js_kvvfs_clear;


  if( globalThis.Storage ){
    /**
       Prior to version 2, kvvfs was only available in the main
       thread.  We retain that for the v1 APIs, exposing them only in
       the main UI thread. As of version 2, kvvfs is available in all
       threads but only via its v2 interface (sqlite3.kvvfs).

       These versions have a default argument value of "" which the v2
       versions lack.
    */
    capi.sqlite3_js_kvvfs_size = (which="")=>sqlite3_js_kvvfs_size(which);
    capi.sqlite3_js_kvvfs_clear = (which="")=>sqlite3_js_kvvfs_clear(which);
  }

  if(sqlite3.oo1?.DB){
    /**
       Functionally equivalent to DB(storageName,'c','kvvfs') except
       that it throws if the given storage name is not one of 'local'
       or 'session'.

       As of version 3.46, the argument may optionally be an options
       object in the form:

       {
         filename: 'session'|'local',
         ... etc. (all options supported by the DB ctor)
       }

       noting that the 'vfs' option supported by main DB
       constructor is ignored here: the vfs is always 'kvvfs'.
    */
    const DB = sqlite3.oo1.DB;
    sqlite3.oo1.JsStorageDb = function(
      storageName = sqlite3.oo1.JsStorageDb.defaultStorageName
    ){
      const opt = DB.dbCtorHelper.normalizeArgs(...arguments);
      opt.vfs = 'kvvfs';
      if( 0 ){
        // Current tests rely on these, but that's arguably a bug
        if( opt.flags ) opt.flags = 'cw'+opt.flags;
        else opt.flags = 'cw';
      }
      switch( opt.filename ){
          /* sqlite3_open(), in these builds, recognizes the names
             below and performs some magic which we want to bypass
             here for sanity's sake. */
        case ":sessionStorage:": opt.filename = 'session'; break;
        case ":localStorage:": opt.filename = 'local'; break;
      }
      const m = /(file:(\/\/)?)([^?]+)/.exec(opt.filename);
      validateStorageName( m ? m[3] : opt.filename);
      DB.dbCtorHelper.call(this, opt);
    };
    sqlite3.oo1.JsStorageDb.defaultStorageName
      = cache.storagePool.session ? 'session' : nameOfThisThreadStorage;
    const jdb = sqlite3.oo1.JsStorageDb;
    jdb.prototype = Object.create(DB.prototype);
    jdb.clearStorage = sqlite3_js_kvvfs_clear;
    /**
       DEPRECATED: the inherited method of this name (as opposed to
       the "static" class method) is deprecated with version 2 of
       kvvfs. This function will, for backwards comaptibility,
       continue to work with localStorage and sessionStorage, but will
       throw for all other storage because they are opened. Version 1
       was not capable of recognizing that the storage was opened so
       permitted wiping it out at any time, but that was arguably a
       bug.

       Clears this database instance's storage or throws if this
       instance has been closed. Returns the number of
       database pages which were cleaned up.
    */
    jdb.prototype.clearStorage = function(){
      return jdb.clearStorage(this.affirmOpen().dbFilename(), true);
    };
    /** Equivalent to sqlite3_js_kvvfs_size(). */
    jdb.storageSize = sqlite3_js_kvvfs_size;
    /**
       Returns the _approximate_ number of bytes this database takes
       up in its storage or throws if this instance has been closed.
    */
    jdb.prototype.storageSize = function(){
      return jdb.storageSize(this.affirmOpen().dbFilename(), true);
    };
  }/*sqlite3.oo1.JsStorageDb*/

  if( sqlite3.__isUnderTest && sqlite3.vtab ){
    /**
       An eponymous vtab for inspecting the kvvfs state.  This is only
       intended for use in testing and development, not part of the
       public API.
    */
    const cols = Object.assign(Object.create(null),{
      rowid:       {type: 'INTEGER'},
      name:        {type: 'TEXT'},
      nRef:        {type: 'INTEGER'},
      nOpen:       {type: 'INTEGER'},
      isTransient: {type: 'INTEGER'},
      dbSize:      {type: 'INTEGER'}
    });
    Object.keys(cols).forEach((v,i)=>cols[v].colId = i);

    const VT = sqlite3.vtab;
    const ProtoCursor = Object.assign(Object.create(null),{
      row: function(){
        return cache.storagePool[this.names[this.rowid]];
      }
    });
    Object.assign(Object.create(ProtoCursor),{
      rowid: 0,
      names: Object.keys(cache.storagePool)
        .filter(v=>!cache.rxJournalSuffix.test(v))
    });
    const cursorState = function(cursor, reset){
      const o = (cursor instanceof capi.sqlite3_vtab_cursor)
            ? cursor
            : VT.xCursor.get(cursor);
      if( reset || !o.vTabState ){
        o.vTabState = Object.assign(Object.create(ProtoCursor),{
          rowid: 0,
          names: Object.keys(cache.storagePool)
            .filter(v=>!cache.rxJournalSuffix.test(v))
        });
      }
      return o.vTabState;
    };

    const dbg = 1 ? ()=>{} : (...args)=>debug("vtab",...args);

    const theModule = function f(){
      return f.mod ??= new sqlite3.capi.sqlite3_module().setupModule({
        catchExceptions: true,
        methods: {
          xConnect: function(pDb, pAux, argc, argv, ppVtab, pzErr){
            dbg("xConnect");
            try{
              const xcol = [];
              Object.keys(cols).forEach((k)=>{
                xcol.push(k+" "+cols[k].type);
              });
              const rc = capi.sqlite3_declare_vtab(
                pDb, "CREATE TABLE ignored("+xcol.join(',')+")"
              );
              if(0===rc){
                const t = VT.xVtab.create(ppVtab);
                util.assert(
                  (t === VT.xVtab.get(wasm.peekPtr(ppVtab))),
                  "output pointer check failed"
                );
              }
              return rc;
            }catch(e){
              return VT.xError('xConnect', e, capi.SQLITE_ERROR);
            }
          },
          xCreate: wasm.ptr.null, // eponymous only
          //xCreate: true, // copy xConnect, i.e. also eponymous only
          xDisconnect: function(pVtab){
            dbg("xDisconnect",...arguments);
            VT.xVtab.dispose(pVtab);
            return 0;
          },
          xOpen: function(pVtab, ppCursor){
            dbg("xOpen",...arguments);
            VT.xCursor.create(ppCursor);
            return 0;
          },
          xClose: function(pCursor){
            dbg("xClose",...arguments);
            const c = VT.xCursor.unget(pCursor);
            delete c.vTabState;
            c.dispose();
            return 0;
          },
          xNext: function(pCursor){
            dbg("xNext",...arguments);
            const c = VT.xCursor.get(pCursor);
            ++cursorState(c).rowid;
            return 0;
          },
          xColumn: function(pCursor, pCtx, iCol){
            dbg("xColumn",...arguments);
            //const c = VT.xCursor.get(pCursor);
            const st = cursorState(pCursor);
            const store = st.row();
            util.assert(store, "Unexpected xColumn call");
            switch(iCol){
              case cols.rowid.colId:
                capi.sqlite3_result_int(pCtx, st.rowid);
                break;
              case cols.name.colId:
                capi.sqlite3_result_text(pCtx, store.jzClass, -1, capi.SQLITE_TRANSIENT);
                break;
              case cols.nRef.colId:
                capi.sqlite3_result_int(pCtx, store.refc);
                break;
              case cols.nOpen.colId:
                capi.sqlite3_result_int(pCtx, store.files.length);
                break;
              case cols.isTransient.colId:
                capi.sqlite3_result_int(pCtx, !!store.deleteAtRefc0);
                break;
              case cols.dbSize.colId:
                capi.sqlite3_result_int(pCtx, storageGetDbSize(store));
                break;
              default:
                capi.sqlite3_result_error(pCtx, "Invalid column id: "+iCol);
                return capi.SQLITE_RANGE;
            }
            return 0;
          },
          xRowid: function(pCursor, ppRowid64){
            dbg("xRowid",...arguments);
            const st = cursorState(pCursor);
            VT.xRowid(ppRowid64, st.rowid);
            return 0;
          },
          xEof: function(pCursor){
            const st = cursorState(pCursor);
            dbg("xEof?="+(!st.row()),...arguments);
            return !st.row();
          },
          xFilter: function(pCursor, idxNum, idxCStr,
                            argc, argv/* [sqlite3_value* ...] */){
            dbg("xFilter",...arguments);
            const st = cursorState(pCursor, true);
            return 0;
          },
          xBestIndex: function(pVtab, pIdxInfo){
            dbg("xBestIndex",...arguments);
            //const t = VT.xVtab.get(pVtab);
            const pii = new capi.sqlite3_index_info(pIdxInfo);
            pii.$estimatedRows = cache.storagePool.size;
            pii.$estimatedCost = 1.0;
            pii.dispose();
            return 0;
          }
        }
      })/*setupModule*/;
    }/*theModule()*/;

    sqlite3.kvvfs.create_module = function(pDb, name="sqlite_kvvfs"){
      return capi.sqlite3_create_module(pDb, name, theModule(),
                                        wasm.ptr.null);
    };

  }/* virtual table */


})/*globalThis.sqlite3ApiBootstrap.initializers*/;
/*
  2026-03-04

  The author disclaims copyright to this source code.  In place of a
  legal notice, here is a blessing:

  *   May you do good and not evil.
  *   May you find forgiveness for yourself and forgive others.
  *   May you share freely, never taking more than you give.

  ***********************************************************************

  This file holds code shared by sqlite3-vfs-opfs{,-wl}.c-pp.js. It
  creates a private/internal sqlite3.opfs namespace common to the two
  and used (only) by them and the test framework. It is not part of
  the public API. The library deletes sqlite3.opfs in its final
  bootstrapping steps unless it's specifically told to keep them (for
  testing purposes only) using an undocumented and unsupported
  mechanism.
*/
globalThis.sqlite3ApiBootstrap.initializers.push(function(sqlite3){
  'use strict';
  if( sqlite3.config.disable?.vfs?.opfs &&
      sqlite3.config.disable.vfs['opfs-vfs'] ){
    return;
  }
  const toss = sqlite3.util.toss,
        capi = sqlite3.capi,
        util = sqlite3.util,
        wasm = sqlite3.wasm;

  /**
     Generic utilities for working with OPFS. This will get filled out
     by the Promise setup and, on success, installed as sqlite3.opfs.

     This is an internal/private namespace intended for use solely by
     the OPFS VFSes and test code for them. The library bootstrapping
     process removes this object in non-testing contexts.
  */
  const opfsUtil = sqlite3.opfs = Object.create(null);

  /**
     Returns true if _this_ thread has access to the OPFS APIs.
  */
  opfsUtil.thisThreadHasOPFS = ()=>{
    return globalThis.FileSystemHandle &&
      globalThis.FileSystemDirectoryHandle &&
      globalThis.FileSystemFileHandle &&
      globalThis.FileSystemFileHandle.prototype.createSyncAccessHandle &&
      navigator?.storage?.getDirectory;
  };

  /**
     Must be called by the OPFS VFSes immediately after they determine
     whether OPFS is available by calling
     thisThreadHasOPFS(). Resolves to the OPFS storage root directory
     and sets opfsUtil.rootDirectory to that value.
  */
  opfsUtil.getRootDir = async function f(){
    return f.promise ??= navigator.storage.getDirectory().then(d=>{
      opfsUtil.rootDirectory = d;
      return d;
    }).catch(e=>{
      delete f.promise;
      throw e;
    });
  };

  /**
     Expects an OPFS file path. It gets resolved, such that ".."
     components are properly expanded, and returned. If the 2nd arg
     is true, the result is returned as an array of path elements,
     else an absolute path string is returned.
  */
  opfsUtil.getResolvedPath = function(filename,splitIt){
    const p = new URL(filename, "file://irrelevant").pathname;
    return splitIt ? p.split('/').filter((v)=>!!v) : p;
  };

  /**
     Takes the absolute path to a filesystem element. Returns an
     array of [handleOfContainingDir, filename]. If the 2nd argument
     is truthy then each directory element leading to the file is
     created along the way. Throws if any creation or resolution
     fails.
  */
  opfsUtil.getDirForFilename = async function f(absFilename, createDirs = false){
    const path = opfsUtil.getResolvedPath(absFilename, true);
    const filename = path.pop();
    let dh = await opfsUtil.getRootDir();
    for(const dirName of path){
      if(dirName){
        dh = await dh.getDirectoryHandle(dirName, {create: !!createDirs});
      }
    }
    return [dh, filename];
  };

  /**
     Creates the given directory name, recursively, in
     the OPFS filesystem. Returns true if it succeeds or the
     directory already exists, else false.
  */
  opfsUtil.mkdir = async function(absDirName){
    try {
      await opfsUtil.getDirForFilename(absDirName+"/filepart", true);
      return true;
    }catch(e){
      //sqlite3.config.warn("mkdir(",absDirName,") failed:",e);
      return false;
    }
  };

  /**
     Checks whether the given OPFS filesystem entry exists,
     returning true if it does, false if it doesn't or if an
     exception is intercepted while trying to make the
     determination.
  */
  opfsUtil.entryExists = async function(fsEntryName){
    try {
      const [dh, fn] = await opfsUtil.getDirForFilename(fsEntryName);
      await dh.getFileHandle(fn);
      return true;
    }catch(e){
      return false;
    }
  };

  /**
     Generates a random ASCII string len characters long, intended for
     use as a temporary file name.
  */
  opfsUtil.randomFilename = function f(len=16){
    if(!f._chars){
      f._chars = "abcdefghijklmnopqrstuvwxyz"+
        "ABCDEFGHIJKLMNOPQRSTUVWXYZ"+
        "012346789";
      f._n = f._chars.length;
    }
    const a = [];
    let i = 0;
    for( ; i < len; ++i){
      const ndx = Math.random() * (f._n * 64) % f._n | 0;
      a[i] = f._chars[ndx];
    }
    return a.join("");
    /*
      An alternative impl. with an unpredictable length
      but much simpler:

      Math.floor(Math.random() * Number.MAX_SAFE_INTEGER).toString(36)
    */
  };

  /**
     Returns a promise which resolves to an object which represents
     all files and directories in the OPFS tree. The top-most object
     has two properties: `dirs` is an array of directory entries
     (described below) and `files` is a list of file names for all
     files in that directory.

     Traversal starts at sqlite3.opfs.rootDirectory.

     Each `dirs` entry is an object in this form:

     ```
     { name: directoryName,
     dirs: [...subdirs],
     files: [...file names]
     }
     ```

     The `files` and `subdirs` entries are always set but may be
     empty arrays.

     The returned object has the same structure but its `name` is
     an empty string. All returned objects are created with
     Object.create(null), so have no prototype.

     Design note: the entries do not contain more information,
     e.g. file sizes, because getting such info is not only
     expensive but is subject to locking-related errors.
  */
  opfsUtil.treeList = async function(){
    const doDir = async function callee(dirHandle,tgt){
      tgt.name = dirHandle.name;
      tgt.dirs = [];
      tgt.files = [];
      for await (const handle of dirHandle.values()){
        if('directory' === handle.kind){
          const subDir = Object.create(null);
          tgt.dirs.push(subDir);
          await callee(handle, subDir);
        }else{
          tgt.files.push(handle.name);
        }
      }
    };
    const root = Object.create(null);
    const dir = await opfsUtil.getRootDir();
    await doDir(dir, root);
    return root;
  };

  /**
     Irrevocably deletes _all_ files in the current origin's OPFS.
     Obviously, this must be used with great caution. It may throw
     an exception if removal of anything fails (e.g. a file is
     locked), but the precise conditions under which the underlying
     APIs will throw are not documented (so we cannot tell you what
     they are).
  */
  opfsUtil.rmfr = async function(){
    const rd = await opfsUtil.getRootDir();
    const dir = rd, opt = {recurse: true};
    for await (const handle of dir.values()){
      dir.removeEntry(handle.name, opt);
    }
  };

  /**
     Deletes the given OPFS filesystem entry.  As this environment
     has no notion of "current directory", the given name must be an
     absolute path. If the 2nd argument is truthy, deletion is
     recursive (use with caution!).

     The returned Promise resolves to true if the deletion was
     successful, else false (but...). The OPFS API reports the
     reason for the failure only in human-readable form, not
     exceptions which can be type-checked to determine the
     failure. Because of that...

     If the final argument is truthy then this function will
     propagate any exception on error, rather than returning false.
  */
  opfsUtil.unlink = async function(fsEntryName, recursive = false,
                                   throwOnError = false){
    try {
      const [hDir, filenamePart] =
            await opfsUtil.getDirForFilename(fsEntryName, false);
      await hDir.removeEntry(filenamePart, {recursive});
      return true;
    }catch(e){
      if(throwOnError){
        throw new Error("unlink(",arguments[0],") failed: "+e.message,{
          cause: e
        });
      }
      return false;
    }
  };

  /**
     Traverses the OPFS filesystem, calling a callback for each
     entry.  The argument may be either a callback function or an
     options object with any of the following properties:

     - `callback`: function which gets called for each filesystem
     entry.  It gets passed 3 arguments: 1) the
     FileSystemFileHandle or FileSystemDirectoryHandle of each
     entry (noting that both are instanceof FileSystemHandle). 2)
     the FileSystemDirectoryHandle of the parent directory. 3) the
     current depth level, with 0 being at the top of the tree
     relative to the starting directory. If the callback returns a
     literal false, as opposed to any other falsy value, traversal
     stops without an error. Any exceptions it throws are
     propagated. Results are undefined if the callback manipulate
     the filesystem (e.g. removing or adding entries) because the
     how OPFS iterators behave in the face of such changes is
     undocumented.

     - `recursive` [bool=true]: specifies whether to recurse into
     subdirectories or not. Whether recursion is depth-first or
     breadth-first is unspecified!

     - `directory` [FileSystemDirectoryEntry=sqlite3.opfs.rootDirectory]
     specifies the starting directory.

     If this function is passed a function, it is assumed to be the
     callback.

     Returns a promise because it has to (by virtue of being async)
     but that promise has no specific meaning: the traversal it
     performs is synchronous. The promise must be used to catch any
     exceptions propagated by the callback, however.
  */
  opfsUtil.traverse = async function(opt){
    const defaultOpt = {
      recursive: true,
      directory: await opfsUtil.getRootDir()
    };
    if('function'===typeof opt){
      opt = {callback:opt};
    }
    opt = Object.assign(defaultOpt, opt||{});
    const doDir = async function callee(dirHandle, depth){
      for await (const handle of dirHandle.values()){
        if(false === opt.callback(handle, dirHandle, depth)) return false;
        else if(opt.recursive && 'directory' === handle.kind){
          if(false === await callee(handle, depth + 1)) break;
        }
      }
    };
    doDir(opt.directory, 0);
  };

  /**
     Impl of opfsUtil.importDb() when it's given a function as its
     second argument.
  */
  const importDbChunked = async function(filename, callback){
    const [hDir, fnamePart] = await opfsUtil.getDirForFilename(filename, true);
    const hFile = await hDir.getFileHandle(fnamePart, {create:true});
    let sah = await hFile.createSyncAccessHandle();
    let nWrote = 0, chunk, checkedHeader = false, err = false;
    try{
      sah.truncate(0);
      while( undefined !== (chunk = await callback()) ){
        if(chunk instanceof ArrayBuffer) chunk = new Uint8Array(chunk);
        if( !checkedHeader && 0===nWrote && chunk.byteLength>=15 ){
          util.affirmDbHeader(chunk);
          checkedHeader = true;
        }
        sah.write(chunk, {at: nWrote});
        nWrote += chunk.byteLength;
      }
      if( nWrote < 512 || 0!==nWrote % 512 ){
        toss("Input size",nWrote,"is not correct for an SQLite database.");
      }
      if( !checkedHeader ){
        const header = new Uint8Array(20);
        sah.read( header, {at: 0} );
        util.affirmDbHeader( header );
      }
      sah.write(new Uint8Array([1,1]), {at: 18}/*force db out of WAL mode*/);
      return nWrote;
    }catch(e){
      await sah.close();
      sah = undefined;
      await hDir.removeEntry( fnamePart ).catch(()=>{});
      throw e;
    }finally {
      if( sah ) await sah.close();
    }
  };

  /**
     Asynchronously imports the given bytes (a byte array or
     ArrayBuffer) into the given database file.

     Results are undefined if the given db name refers to an opened
     db.

     If passed a function for its second argument, its behaviour
     changes: imports its data in chunks fed to it by the given
     callback function. It calls the callback (which may be async)
     repeatedly, expecting either a Uint8Array or ArrayBuffer (to
     denote new input) or undefined (to denote EOF). For so long as
     the callback continues to return non-undefined, it will append
     incoming data to the given VFS-hosted database file. When
     called this way, the resolved value of the returned Promise is
     the number of bytes written to the target file.

     It very specifically requires the input to be an SQLite3
     database and throws if that's not the case.  It does so in
     order to prevent this function from taking on a larger scope
     than it is specifically intended to. i.e. we do not want it to
     become a convenience for importing arbitrary files into OPFS.

     This routine rewrites the database header bytes in the output
     file (not the input array) to force disabling of WAL mode.

     On error this throws and the state of the input file is
     undefined (it depends on where the exception was triggered).

     On success, resolves to the number of bytes written.
  */
  opfsUtil.importDb = async function(filename, bytes){
    if( bytes instanceof Function ){
      return importDbChunked(filename, bytes);
    }
    if(bytes instanceof ArrayBuffer) bytes = new Uint8Array(bytes);
    util.affirmIsDb(bytes);
    const n = bytes.byteLength;
    const [hDir, fnamePart] = await opfsUtil.getDirForFilename(filename, true);
    let sah, err, nWrote = 0;
    try {
      const hFile = await hDir.getFileHandle(fnamePart, {create:true});
      sah = await hFile.createSyncAccessHandle();
      sah.truncate(0);
      nWrote = sah.write(bytes, {at: 0});
      if(nWrote != n){
        toss("Expected to write "+n+" bytes but wrote "+nWrote+".");
      }
      sah.write(new Uint8Array([1,1]), {at: 18}) /* force db out of WAL mode */;
      return nWrote;
    }catch(e){
      if( sah ){ await sah.close(); sah = undefined; }
      await hDir.removeEntry( fnamePart ).catch(()=>{});
      throw e;
    }finally{
      if( sah ) await sah.close();
    }
  };

  /**
     Checks for features required for OPFS VFSes and throws with a
     descriptive error message if they're not found. This is intended
     to be run as part of async VFS installation steps.
  */
  opfsUtil.vfsInstallationFeatureCheck = function(vfsName){
    if( !globalThis.SharedArrayBuffer || !globalThis.Atomics ){
      toss("Cannot install OPFS: Missing SharedArrayBuffer and/or Atomics.",
           "The server must emit the COOP/COEP response headers to enable those.",
           "See https://sqlite.org/wasm/doc/trunk/persistence.md#coop-coep");
    }else if( 'undefined'===typeof WorkerGlobalScope ){
      toss("The OPFS sqlite3_vfs cannot run in the main thread",
           "because it requires Atomics.wait().");
    }else if( !globalThis.FileSystemHandle ||
              !globalThis.FileSystemDirectoryHandle ||
              !globalThis.FileSystemFileHandle?.prototype?.createSyncAccessHandle ||
              !navigator?.storage?.getDirectory ){
      toss("Missing required OPFS APIs.");
    }else if( 'opfs-wl'===vfsName && !globalThis.Atomics.waitAsync ){
      toss('The',vfsName,'VFS requires Atomics.waitAsync(), which is not available.');
    }
  };

  /**
     Must be called by the VFS's main installation routine and passed
     the options object that function receives and a reference to that
     function itself (we don't need this anymore).

     It throws if OPFS is not available.

     If it returns falsy, it detected that OPFS should be disabled, in
     which case the callee should immediately return/resolve to the
     sqlite3 object.

     Else it returns a new copy of the options object, fleshed out
     with any missing defaults. The caller must:

     - Set up any local state they need.

     - Call opfsUtil.createVfsState(vfsName,opt), where opt is the
     object returned by this function.

     - Set up any references they may need to state returned
     by the previous step.

     - Call opfvs.bindVfs()
  */
  opfsUtil.initOptions = function callee(vfsName, options){
    const urlParams = new URL(globalThis.location.href).searchParams;
    if( urlParams.has(vfsName+'-disable') ){
      //sqlite3.config.warn('Explicitly not installing "opfs" VFS due to opfs-disable flag.');
      return;
    }
    try{
      opfsUtil.vfsInstallationFeatureCheck(vfsName);
    }catch(e){
      return;
    }
    options = util.nu(options);
    options.vfsName = vfsName;
    options.verbose ??= urlParams.has('opfs-verbose')
      ? +urlParams.get('opfs-verbose') : 1;
    options.sanityChecks ??= urlParams.has('opfs-sanity-check');

    if( !opfsUtil.proxyUri ){
      opfsUtil.proxyUri = "sqlite3-opfs-async-proxy.js";
      if( sqlite3.scriptInfo?.sqlite3Dir ){
        /* Doing this from one scope up, outside of this function, does
           not work. */
        opfsUtil.proxyUri = (
          sqlite3.scriptInfo.sqlite3Dir + opfsUtil.proxyUri
        );
      }
    }
    options.proxyUri ??= opfsUtil.proxyUri;
    if('function' === typeof options.proxyUri){
      options.proxyUri = options.proxyUri();
    }
    //sqlite3.config.warn("opfsUtil options =",JSON.stringify(options), 'urlParams =', urlParams);
    return opfsUtil.options = options;
  };

  /**
     Creates, populates, and returns the main state object used by the
     "opfs" and "opfs-wl" VFSes, and transfered from those to their
     async counterparts.

     The returned object's vfs property holds the fully-populated
     capi.sqlite3_vfs instance, tagged with lots of extra state which
     the current VFSes need to have exposed to them.

     After setting up any local state needed, the caller must call
     theVfs.bindVfs(X,Y), where X is an object containing the
     sqlite3_io_methods to override and Y is a callback which gets
     triggered if init succeeds, before the final Promise decides
     whether or not to reject.

     This object must, when it's passed to the async part, contain
     only cloneable or sharable objects. After the worker's "inited"
     message arrives, other types of data may be added to it.
  */
  opfsUtil.createVfsState = function(){
    const state = util.nu();
    const options = opfsUtil.options;
    state.verbose = options.verbose;

    const loggers = [
      sqlite3.config.error,
      sqlite3.config.warn,
      sqlite3.config.log
    ];
    const vfsName = options.vfsName
          || toss("Maintenance required: missing VFS name");
    const logImpl = (level,...args)=>{
      if(state.verbose>level) loggers[level](vfsName+":",...args);
    };
    const log   = (...args)=>logImpl(2, ...args),
          warn  = (...args)=>logImpl(1, ...args),
          error = (...args)=>logImpl(0, ...args),
          capi  = sqlite3.capi,
          wasm  = sqlite3.wasm;

    const opfsVfs = state.vfs = new capi.sqlite3_vfs();
    const opfsIoMethods = opfsVfs.ioMethods = new capi.sqlite3_io_methods();

    opfsIoMethods.$iVersion = 1;
    opfsVfs.$iVersion = 2/*yes, two*/;
    opfsVfs.$szOsFile = capi.sqlite3_file.structInfo.sizeof;
    opfsVfs.$mxPathname = 1024/* sure, why not? The OPFS name length limit
                                 is undocumented/unspecified. */;
    opfsVfs.$zName = wasm.allocCString(vfsName);
    opfsVfs.addOnDispose(
      '$zName', opfsVfs.$zName, opfsIoMethods
      /**
         Pedantic sidebar: the entries in this array are items to
         clean up when opfsVfs.dispose() is called, but in this
         environment it will never be called. The VFS instance simply
         hangs around until the WASM module instance is cleaned up. We
         "could" _hypothetically_ clean it up by "importing" an
         sqlite3_os_end() impl into the wasm build, but the shutdown
         order of the wasm engine and the JS one are undefined so
         there is no guaranty that the opfsVfs instance would be
         available in one environment or the other when
         sqlite3_os_end() is called (_if_ it gets called at all in a
         wasm build, which is undefined). i.e. addOnDispose() here is
         a matter of "correctness", not necessity. It just wouldn't do
         to leave the impression that we're blindly leaking memory.
      */
    );

    opfsVfs.metrics = util.nu({
      counters: util.nu(),
      dump: function(){
        let k, n = 0, t = 0, w = 0;
        for(k in state.opIds){
          const m = metrics[k];
          n += m.count;
          t += m.time;
          w += m.wait;
          m.avgTime = (m.count && m.time) ? (m.time / m.count) : 0;
          m.avgWait = (m.count && m.wait) ? (m.wait / m.count) : 0;
        }
        sqlite3.config.log(globalThis.location.href,
                    "metrics for",globalThis.location.href,":",metrics,
                    "\nTotal of",n,"op(s) for",t,
                    "ms (incl. "+w+" ms of waiting on the async side)");
        sqlite3.config.log("Serialization metrics:",opfsVfs.metrics.counters.s11n);
        opfsVfs.worker?.postMessage?.({type:'opfs-async-metrics'});
      },
      reset: function(){
        let k;
        const r = (m)=>(m.count = m.time = m.wait = 0);
        const m = opfsVfs.metrics.counters;
        for(k in state.opIds){
          r(m[k] = Object.create(null));
        }
        let s = m.s11n = Object.create(null);
        s = s.serialize = Object.create(null);
        s.count = s.time = 0;
        s = m.s11n.deserialize = Object.create(null);
        s.count = s.time = 0;
      }
    })/*opfsVfs.metrics*/;

    /**
       asyncIdleWaitTime is how long (ms) to wait, in the async proxy,
       for each Atomics.wait() when waiting on inbound VFS API calls.
       We need to wake up periodically to give the thread a chance to
       do other things. If this is too high (e.g. 500ms) then even two
       workers/tabs can easily run into locking errors. Some multiple
       of this value is also used for determining how long to wait on
       lock contention to free up.
    */
    state.asyncIdleWaitTime = 150;

    /**
       Whether the async counterpart should log exceptions to
       the serialization channel. That produces a great deal of
       noise for seemingly innocuous things like xAccess() checks
       for missing files, so this option may have one of 3 values:

       0 = no exception logging.

       1 = only log exceptions for "significant" ops like xOpen(),
       xRead(), and xWrite(). Exceptions related to, e.g., wait/retry
       loops in acquiring SyncAccessHandles are not logged.

       2 = log all exceptions.
    */
    state.asyncS11nExceptions = 1;
    /* Size of file I/O buffer block. 64k = max sqlite3 page size, and
       xRead/xWrite() will never deal in blocks larger than that. */
    state.fileBufferSize = 1024 * 64;
    state.sabS11nOffset = state.fileBufferSize;
    /**
       The size of the block in our SAB for serializing arguments and
       result values. Needs to be large enough to hold serialized
       values of any of the proxied APIs. Filenames are the largest
       part but are limited to opfsVfs.$mxPathname bytes. We also
       store exceptions there, so it needs to be long enough to hold
       a reasonably long exception string.
    */
    state.sabS11nSize = opfsVfs.$mxPathname * 2;
    /**
       The SAB used for all data I/O between the synchronous and
       async halves (file i/o and arg/result s11n).
    */
    state.sabIO = new SharedArrayBuffer(
      state.fileBufferSize/* file i/o block */
      + state.sabS11nSize/* argument/result serialization block */
    );

    /**
       For purposes of Atomics.wait() and Atomics.notify(), we use a
       SharedArrayBuffer with one slot reserved for each of the API
       proxy's methods. The sync side of the API uses Atomics.wait()
       on the corresponding slot and the async side uses
       Atomics.notify() on that slot. state.opIds holds the SAB slot
       IDs of each of those.
    */
    state.opIds = Object.create(null);
    {
      /* Indexes for use in our SharedArrayBuffer... */
      let i = 0;
      /* SAB slot used to communicate which operation is desired
         between both workers. This worker writes to it and the other
         listens for changes and clears it. The values written to it
         are state.opIds.x[A-Z][a-z]+, defined below.*/
      state.opIds.whichOp = i++;
      /* Slot for storing return values. This side listens to that
         slot and the async proxy writes to it. */
      state.opIds.rc = i++;
      /* Each function gets an ID which this worker writes to the
         state.opIds.whichOp slot. The async-api worker uses
         Atomic.wait() on the whichOp slot to figure out which
         operation to run next. */
      state.opIds.xAccess = i++;
      state.opIds.xClose = i++;
      state.opIds.xDelete = i++;
      state.opIds.xDeleteNoWait = i++;
      state.opIds.xFileSize = i++;
      state.opIds.xLock = i++;
      state.opIds.xOpen = i++;
      state.opIds.xRead = i++;
      state.opIds.xSleep = i++;
      state.opIds.xSync = i++;
      state.opIds.xTruncate = i++;
      state.opIds.xUnlock = i++;
      state.opIds.xWrite = i++;
      state.opIds.mkdir = i++ /*currently unused*/;
      /** Internal signals which are used only during development and
          testing via the dev console. */
      state.opIds['opfs-async-metrics'] = i++;
      state.opIds['opfs-async-shutdown'] = i++;
      /* The retry slot is used by the async part for wait-and-retry
         semantics. It is never written to, only used as a convenient
         place to wait-with-timeout for a value which will never be
         written, i.e. sleep()ing, before retrying a failed attempt to
         acquire a SharedAccessHandle. */
      state.opIds.retry = i++;
      state.sabOP = new SharedArrayBuffer(
        i * 4/* 4==sizeof int32, noting that Atomics.wait() and
                friends can only function on Int32Array views of an
                SAB. */);
    }
    /**
       SQLITE_xxx constants to export to the async worker
       counterpart...
    */
    state.sq3Codes = Object.create(null);
    for(const k of [
      'SQLITE_ACCESS_EXISTS',
      'SQLITE_ACCESS_READWRITE',
      'SQLITE_BUSY',
      'SQLITE_CANTOPEN',
      'SQLITE_ERROR',
      'SQLITE_IOERR',
      'SQLITE_IOERR_ACCESS',
      'SQLITE_IOERR_CLOSE',
      'SQLITE_IOERR_DELETE',
      'SQLITE_IOERR_FSYNC',
      'SQLITE_IOERR_LOCK',
      'SQLITE_IOERR_READ',
      'SQLITE_IOERR_SHORT_READ',
      'SQLITE_IOERR_TRUNCATE',
      'SQLITE_IOERR_UNLOCK',
      'SQLITE_IOERR_WRITE',
      'SQLITE_LOCK_EXCLUSIVE',
      'SQLITE_LOCK_NONE',
      'SQLITE_LOCK_PENDING',
      'SQLITE_LOCK_RESERVED',
      'SQLITE_LOCK_SHARED',
      'SQLITE_LOCKED',
      'SQLITE_MISUSE',
      'SQLITE_NOTFOUND',
      'SQLITE_OPEN_CREATE',
      'SQLITE_OPEN_DELETEONCLOSE',
      'SQLITE_OPEN_MAIN_DB',
      'SQLITE_OPEN_READONLY',
      'SQLITE_LOCK_NONE',
      'SQLITE_LOCK_SHARED',
      'SQLITE_LOCK_RESERVED',
      'SQLITE_LOCK_PENDING',
      'SQLITE_LOCK_EXCLUSIVE'
    ]){
      state.sq3Codes[k] =
        capi[k] ?? toss("Maintenance required: not found:",k);
    }

    state.opfsFlags = Object.assign(Object.create(null),{
      /**
         Flag for use with xOpen(). URI flag "opfs-unlock-asap=1"
         enables this. See defaultUnlockAsap, below.
      */
      OPFS_UNLOCK_ASAP: 0x01,
      /**
         Flag for use with xOpen(). URI flag "delete-before-open=1"
         tells the VFS to delete the db file before attempting to open
         it. This can be used, e.g., to replace a db which has been
         corrupted (without forcing us to expose a delete/unlink()
         function in the public API).

         Failure to unlink the file is ignored but may lead to
         downstream errors.  An unlink can fail if, e.g., another tab
         has the handle open.

         It goes without saying that deleting a file out from under
         another instance results in Undefined Behavior.
      */
      OPFS_UNLINK_BEFORE_OPEN: 0x02,
      /**
         If true, any async routine which must implicitly acquire a
         sync access handle (i.e. an OPFS lock), without an active
         xLock(), will release that lock at the end of the call which
         acquires it. If false, such implicit locks are not released
         until the VFS is idle for some brief amount of time, as
         defined by state.asyncIdleWaitTime.

         The benefit of enabling this is higher concurrency. The
         down-side is much-reduced performance (as much as a 4x
         decrease in speedtest1).
      */
      defaultUnlockAsap: false
    });

    opfsVfs.metrics.reset()/*must not be called until state.opIds is set up*/;
    const metrics = opfsVfs.metrics.counters;

    /**
       Runs the given operation (by name) in the async worker
       counterpart, waits for its response, and returns the result
       which the async worker writes to SAB[state.opIds.rc]. The 2nd
       and subsequent arguments must be the arguments for the async op
       (see sqlite3-opfs-async-proxy.c-pp.js).
    */
    const opRun = opfsVfs.opRun = (op,...args)=>{
      const opNdx = state.opIds[op] || toss(opfsVfs.vfsName+": Invalid op ID:",op);
      state.s11n.serialize(...args);
      Atomics.store(state.sabOPView, state.opIds.rc, -1);
      Atomics.store(state.sabOPView, state.opIds.whichOp, opNdx);
      Atomics.notify(state.sabOPView, state.opIds.whichOp)
      /* async thread will take over here */;
      const t = performance.now();
      while('not-equal'!==Atomics.wait(state.sabOPView, state.opIds.rc, -1)){
        /*
          The reason for this loop is buried in the details of a long
          discussion at:

          https://github.com/sqlite/sqlite-wasm/issues/12

          Summary: in at least one browser flavor, under high loads,
          the wait()/notify() pairings can get out of sync and/or
          spuriously wake up. Calling wait() here until it returns
          'not-equal' gets them back in sync.
        */
      }
      /* When the above wait() call returns 'not-equal', the async
         half will have completed the operation and reported its
         results in the state.opIds.rc slot of the SAB. It may have
         also serialized an exception for us. */
      const rc = Atomics.load(state.sabOPView, state.opIds.rc);
      metrics[op].wait += performance.now() - t;
      if(rc && state.asyncS11nExceptions){
        const err = state.s11n.deserialize();
        if(err) error(op+"() async error:",...err);
      }
      return rc;
    };

    const opTimer = Object.create(null);
    opTimer.op = undefined;
    opTimer.start = undefined;
    const mTimeStart = opfsVfs.mTimeStart = (op)=>{
      opTimer.start = performance.now();
      opTimer.op = op;
      ++metrics[op].count;
    };
    const mTimeEnd = opfsVfs.mTimeEnd = ()=>(
      metrics[opTimer.op].time += performance.now() - opTimer.start
    );

    /**
       Map of sqlite3_file pointers to objects constructed by xOpen().
    */
    const __openFiles = opfsVfs.__openFiles = Object.create(null);

    /**
       Impls for the sqlite3_io_methods methods. Maintenance reminder:
       members are in alphabetical order to simplify finding them.
    */
    const ioSyncWrappers = opfsVfs.ioSyncWrappers = util.nu({
      xCheckReservedLock: function(pFile,pOut){
        /**
           After consultation with a topic expert: "opfs-wl" will
           continue to use the same no-op impl which "opfs" does
           because:

           - xCheckReservedLock() is just a hint. If SQLite needs to
           lock, it's still going to try to lock.

           - We cannot do this check synchronously in "opfs-wl",
           so would need to pass it to the async proxy. That would
           make it inordinately expensive considering that it's
           just a hint.
        */
        wasm.poke(pOut, 0, 'i32');
        return 0;
      },
      xClose: function(pFile){
        mTimeStart('xClose');
        let rc = 0;
        const f = __openFiles[pFile];
        if(f){
          delete __openFiles[pFile];
          rc = opRun('xClose', pFile);
          if(f.sq3File) f.sq3File.dispose();
        }
        mTimeEnd();
        return rc;
      },
      xDeviceCharacteristics: function(pFile){
        return capi.SQLITE_IOCAP_UNDELETABLE_WHEN_OPEN;
      },
      xFileControl: function(pFile, opId, pArg){
        /*mTimeStart('xFileControl');
         mTimeEnd();*/
        return capi.SQLITE_NOTFOUND;
      },
      xFileSize: function(pFile,pSz64){
        mTimeStart('xFileSize');
        let rc = opRun('xFileSize', pFile);
        if(0==rc){
          try {
            const sz = state.s11n.deserialize()[0];
            wasm.poke(pSz64, sz, 'i64');
          }catch(e){
            error("Unexpected error reading xFileSize() result:",e);
            rc = state.sq3Codes.SQLITE_IOERR;
          }
        }
        mTimeEnd();
        return rc;
      },
      xRead: function(pFile,pDest,n,offset64){
       mTimeStart('xRead');
        const f = __openFiles[pFile];
        let rc;
        try {
          rc = opRun('xRead',pFile, n, Number(offset64));
          if(0===rc || capi.SQLITE_IOERR_SHORT_READ===rc){
            /**
               Results get written to the SharedArrayBuffer f.sabView.
               Because the heap is _not_ a SharedArrayBuffer, we have
               to copy the results. TypedArray.set() seems to be the
               fastest way to copy this. */
            wasm.heap8u().set(f.sabView.subarray(0, n), Number(pDest));
          }
        }catch(e){
          error("xRead(",arguments,") failed:",e,f);
          rc = capi.SQLITE_IOERR_READ;
        }
        mTimeEnd();
        return rc;
      },
      xSync: function(pFile,flags){
        mTimeStart('xSync');
        const rc = opRun('xSync', pFile, flags);
        mTimeEnd();
        return rc;
      },
      xTruncate: function(pFile,sz64){
        mTimeStart('xTruncate');
        const rc = opRun('xTruncate', pFile, Number(sz64));
        mTimeEnd();
        return rc;
      },
      xWrite: function(pFile,pSrc,n,offset64){
        mTimeStart('xWrite');
        const f = __openFiles[pFile];
        let rc;
        try {
          f.sabView.set(wasm.heap8u().subarray(
            Number(pSrc), Number(pSrc) + n
          ));
          rc = opRun('xWrite', pFile, n, Number(offset64));
        }catch(e){
          error("xWrite(",arguments,") failed:",e,f);
          rc = capi.SQLITE_IOERR_WRITE;
        }
        mTimeEnd();
        return rc;
      }
    })/*ioSyncWrappers*/;

    /**
       Impls for the sqlite3_vfs methods. Maintenance reminder: members
       are in alphabetical order to simplify finding them.
    */
    const vfsSyncWrappers = opfsVfs.vfsSyncWrappers = {
      xAccess: function(pVfs,zName,flags,pOut){
       mTimeStart('xAccess');
        const rc = opRun('xAccess', wasm.cstrToJs(zName));
        wasm.poke( pOut, (rc ? 0 : 1), 'i32' );
        mTimeEnd();
        return 0;
      },
      xCurrentTime: function(pVfs,pOut){
        wasm.poke(pOut, 2440587.5 + (new Date().getTime()/86400000),
                  'double');
        return 0;
      },
      xCurrentTimeInt64: function(pVfs,pOut){
        wasm.poke(pOut, (2440587.5 * 86400000) + new Date().getTime(),
                  'i64');
        return 0;
      },
      xDelete: function(pVfs, zName, doSyncDir){
        mTimeStart('xDelete');
        const rc = opRun('xDelete', wasm.cstrToJs(zName), doSyncDir, false);
        mTimeEnd();
        return rc;
      },
      xFullPathname: function(pVfs,zName,nOut,pOut){
        /* Until/unless we have some notion of "current dir"
           in OPFS, simply copy zName to pOut... */
        const i = wasm.cstrncpy(pOut, zName, nOut);
        return i<nOut ? 0 : capi.SQLITE_CANTOPEN
        /*CANTOPEN is required by the docs but SQLITE_RANGE would be a closer match*/;
      },
      xGetLastError: function(pVfs,nOut,pOut){
        /* Mutex use in the overlying APIs cause xGetLastError() to
           not be terribly useful for us. e.g. it can't be used to
           convey error messages from xOpen() because there would be a
           race condition between sqlite3_open()'s call to xOpen() and
           this function. */
        sqlite3.config.warn("OPFS xGetLastError() has nothing sensible to return.");
        return 0;
      },
      //xSleep is optionally defined below
      xOpen: function f(pVfs, zName, pFile, flags, pOutFlags){
        mTimeStart('xOpen');
        let opfsFlags = 0;
        let jzName, zToFree;
        if( !zName ){
          jzName = opfsUtil.randomFilename();
          zName = zToFree = wasm.allocCString(jzName);
        }else if(wasm.isPtr(zName)){
          if(capi.sqlite3_uri_boolean(zName, "opfs-unlock-asap", 0)){
            /* -----------------------^^^^^ MUST pass the untranslated
               C-string here. */
            opfsFlags |= state.opfsFlags.OPFS_UNLOCK_ASAP;
          }
          if(capi.sqlite3_uri_boolean(zName, "delete-before-open", 0)){
            opfsFlags |= state.opfsFlags.OPFS_UNLINK_BEFORE_OPEN;
          }
          jzName = wasm.cstrToJs(zName);
          //sqlite3.config.warn("xOpen zName =",zName, "opfsFlags =",opfsFlags);
        }else{
          sqlite3.config.error("Impossible zName value in xOpen?", zName);
          return capi.SQLITE_CANTOPEN;
        }
        const fh = util.nu({
          fid: pFile,
          filename: jzName,
          sab: new SharedArrayBuffer(state.fileBufferSize),
          flags: flags,
          readOnly: !(capi.SQLITE_OPEN_CREATE & flags)
            && !!(flags & capi.SQLITE_OPEN_READONLY)
        });
        const rc = opRun('xOpen', pFile, jzName, flags, opfsFlags);
        if(rc){
          if( zToFree ) wasm.dealloc(zToFree);
        }else{
          /* Recall that sqlite3_vfs::xClose() will be called, even on
             error, unless pFile->pMethods is NULL. */
          if(fh.readOnly){
            wasm.poke(pOutFlags, capi.SQLITE_OPEN_READONLY, 'i32');
          }
          __openFiles[pFile] = fh;
          fh.sabView = state.sabFileBufView;
          fh.sq3File = new capi.sqlite3_file(pFile);
          if( zToFree ) fh.sq3File.addOnDispose(zToFree);
          fh.sq3File.$pMethods = opfsIoMethods.pointer;
          fh.lockType = capi.SQLITE_LOCK_NONE;
        }
        mTimeEnd();
        return rc;
      }/*xOpen()*/
    }/*vfsSyncWrappers*/;

    const pDVfs = capi.sqlite3_vfs_find(null)/*pointer to default VFS*/;
    if(pDVfs){
      const dVfs = new capi.sqlite3_vfs(pDVfs);
      opfsVfs.$xRandomness = dVfs.$xRandomness;
      opfsVfs.$xSleep = dVfs.$xSleep;
      dVfs.dispose();
    }
    if(!opfsVfs.$xRandomness){
      /* If the default VFS has no xRandomness(), add a basic JS impl... */
      opfsVfs.vfsSyncWrappers.xRandomness = function(pVfs, nOut, pOut){
        const heap = wasm.heap8u();
        let i = 0;
        const npOut = Number(pOut);
        for(; i < nOut; ++i) heap[npOut + i] = (Math.random()*255000) & 0xFF;
        return i;
      };
    }
    if(!opfsVfs.$xSleep){
      /* If we can inherit an xSleep() impl from the default VFS then
         assume it's sane and use it, otherwise install a JS-based
         one. */
      opfsVfs.vfsSyncWrappers.xSleep = function(pVfs,ms){
        mTimeStart('xSleep');
        Atomics.wait(state.sabOPView, state.opIds.xSleep, 0, ms);
        mTimeEnd();
        return 0;
      };
    }

const initS11n = function(){
  /**
     This proxy de/serializes cross-thread function arguments and
     output-pointer values via the state.sabIO SharedArrayBuffer,
     using the region defined by (state.sabS11nOffset,
     state.sabS11nOffset + state.sabS11nSize]. Only one dataset is
     recorded at a time.

     This is not a general-purpose format. It only supports the
     range of operations, and data sizes, needed by the
     sqlite3_vfs and sqlite3_io_methods operations. Serialized
     data are transient and this serialization algorithm may
     change at any time.

     The data format can be succinctly summarized as:

     Nt...Td...D

     Where:

     - N = number of entries (1 byte)

     - t = type ID of first argument (1 byte)

     - ...T = type IDs of the 2nd and subsequent arguments (1 byte
     each).

     - d = raw bytes of first argument (per-type size).

     - ...D = raw bytes of the 2nd and subsequent arguments (per-type
     size).

     All types except strings have fixed sizes. Strings are stored
     using their TextEncoder/TextDecoder representations. It would
     arguably make more sense to store them as Int16Arrays of
     their JS character values, but how best/fastest to get that
     in and out of string form is an open point. Initial
     experimentation with that approach did not gain us any speed.

     Historical note: this impl was initially about 1% this size by
     using using JSON.stringify/parse(), but using fit-to-purpose
     serialization saves considerable runtime.
  */
  if(state.s11n) return state.s11n;
  const textDecoder = new TextDecoder(),
        textEncoder = new TextEncoder('utf-8'),
        viewU8 = new Uint8Array(state.sabIO, state.sabS11nOffset, state.sabS11nSize),
        viewDV = new DataView(state.sabIO, state.sabS11nOffset, state.sabS11nSize);
  state.s11n = Object.create(null);
  /* Only arguments and return values of these types may be
     serialized. This covers the whole range of types needed by the
     sqlite3_vfs API. */
  const TypeIds = Object.create(null);
  TypeIds.number  = { id: 1, size: 8, getter: 'getFloat64', setter: 'setFloat64' };
  TypeIds.bigint  = { id: 2, size: 8, getter: 'getBigInt64', setter: 'setBigInt64' };
  TypeIds.boolean = { id: 3, size: 4, getter: 'getInt32', setter: 'setInt32' };
  TypeIds.string =  { id: 4 };

  const getTypeId = (v)=>(
    TypeIds[typeof v]
      || toss("Maintenance required: this value type cannot be serialized.",v)
  );
  const getTypeIdById = (tid)=>{
    switch(tid){
      case TypeIds.number.id: return TypeIds.number;
      case TypeIds.bigint.id: return TypeIds.bigint;
      case TypeIds.boolean.id: return TypeIds.boolean;
      case TypeIds.string.id: return TypeIds.string;
      default: toss("Invalid type ID:",tid);
    }
  };

  /**
     Returns an array of the deserialized state stored by the most
     recent serialize() operation (from this thread or the
     counterpart thread), or null if the serialization buffer is
     empty.  If passed a truthy argument, the serialization buffer
     is cleared after deserialization.
  */
  state.s11n.deserialize = function(clear=false){
    const t = performance.now();
    const argc = viewU8[0];
    const rc = argc ? [] : null;
    if(argc){
      const typeIds = [];
      let offset = 1, i, n, v;
      for(i = 0; i < argc; ++i, ++offset){
        typeIds.push(getTypeIdById(viewU8[offset]));
      }
      for(i = 0; i < argc; ++i){
        const t = typeIds[i];
        if(t.getter){
          v = viewDV[t.getter](offset, state.littleEndian);
          offset += t.size;
        }else{/*String*/
          n = viewDV.getInt32(offset, state.littleEndian);
          offset += 4;
          v = textDecoder.decode(viewU8.slice(offset, offset+n));
          offset += n;
        }
        rc.push(v);
      }
    }
    if(clear) viewU8[0] = 0;
    //log("deserialize:",argc, rc);
    return rc;
  };

  /**
     Serializes all arguments to the shared buffer for consumption
     by the counterpart thread.

     This routine is only intended for serializing OPFS VFS
     arguments and (in at least one special case) result values,
     and the buffer is sized to be able to comfortably handle
     those.

     If passed no arguments then it zeroes out the serialization
     state.
  */
  state.s11n.serialize = function(...args){
    const t = performance.now();
    if(args.length){
      //log("serialize():",args);
      const typeIds = [];
      let i = 0, offset = 1;
      viewU8[0] = args.length & 0xff /* header = # of args */;
      for(; i < args.length; ++i, ++offset){
        /* Write the TypeIds.id value into the next args.length
           bytes. */
        typeIds.push(getTypeId(args[i]));
        viewU8[offset] = typeIds[i].id;
      }
      for(i = 0; i < args.length; ++i) {
        /* Deserialize the following bytes based on their
           corresponding TypeIds.id from the header. */
        const t = typeIds[i];
        if(t.setter){
          viewDV[t.setter](offset, args[i], state.littleEndian);
          offset += t.size;
        }else{/*String*/
          const s = textEncoder.encode(args[i]);
          viewDV.setInt32(offset, s.byteLength, state.littleEndian);
          offset += 4;
          viewU8.set(s, offset);
          offset += s.byteLength;
        }
      }
      //log("serialize() result:",viewU8.slice(0,offset));
    }else{
      viewU8[0] = 0;
    }
  };


  return state.s11n;
}/*initS11n()*/;
    opfsVfs.initS11n = initS11n;

    /**
       To be called by the VFS's main installation routine after it has
       wired up enough state to provide its overridden io-method impls
       (which must be properties of the ioMethods argument). Returns a
       Promise which the installation routine must return. callback must
       be a function which performs any post-bootstrap touchups, namely
       plugging in a sqlite3.oo1 wrapper. It is passed (sqlite3, opfsVfs),
       where opfsVfs is the sqlite3_vfs object which was set up by
       opfsUtil.createVfsState().
    */
    opfsVfs.bindVfs = function(ioMethods, callback){
      Object.assign(opfsVfs.ioSyncWrappers, ioMethods);
      const thePromise = new Promise(function(promiseResolve_, promiseReject_){
        let promiseWasRejected = undefined;
        const promiseReject = (err)=>{
          promiseWasRejected = true;
          opfsVfs.dispose();
          return promiseReject_(err);
        };
        const promiseResolve = ()=>{
          try{
            callback(sqlite3, opfsVfs);
          }catch(e){
            return promiseReject(e);
          }
          promiseWasRejected = false;
          return promiseResolve_(sqlite3);
        };
        const options = opfsUtil.options;
        const proxyUri = options.proxyUri +(
          (options.proxyUri.indexOf('?')<0) ? '?' : '&'
        )+'vfs='+vfsName;
        //sqlite3.config.error("proxyUri",options.proxyUri, (new Error()));
        const W = opfsVfs.worker =
        new Worker(new URL(proxyUri, import.meta.url));
        let zombieTimer = setTimeout(()=>{
          /* At attempt to work around a browser-specific quirk in which
             the Worker load is failing in such a way that we neither
             resolve nor reject it. This workaround gives that resolve/reject
             a time limit and rejects if that timer expires. Discussion:
             https://sqlite.org/forum/forumpost/a708c98dcb3ef */
          if(undefined===promiseWasRejected){
            promiseReject(
              new Error("Timeout while waiting for OPFS async proxy worker.")
            );
          }
        }, 4000);
        W._originalOnError = W.onerror /* will be restored later */;
        W.onerror = function(err){
          // The error object doesn't contain any useful info when the
          // failure is, e.g., that the remote script is 404.
          error("Error initializing OPFS asyncer:",err);
          promiseReject(new Error("Loading OPFS async Worker failed for unknown reasons."));
        };

        const opRun = opfsVfs.opRun;

        const sanityCheck = function(){
          const scope = wasm.scopedAllocPush();
          const sq3File = new capi.sqlite3_file();
          try{
            const fid = sq3File.pointer;
            const openFlags = capi.SQLITE_OPEN_CREATE
                  | capi.SQLITE_OPEN_READWRITE
            //| capi.SQLITE_OPEN_DELETEONCLOSE
                  | capi.SQLITE_OPEN_MAIN_DB;
            const pOut = wasm.scopedAlloc(8);
            const dbFile = "/sanity/check/file"+randomFilename(8);
            const zDbFile = wasm.scopedAllocCString(dbFile);
            let rc;
            state.s11n.serialize("This is ä string.");
            rc = state.s11n.deserialize();
            log("deserialize() says:",rc);
            if("This is ä string."!==rc[0]) toss("String d13n error.");
            opfsVfs.vfsSyncWrappers.xAccess(opfsVfs.pointer, zDbFile, 0, pOut);
            rc = wasm.peek(pOut,'i32');
            log("xAccess(",dbFile,") exists ?=",rc);
            rc = opfsVfs.vfsSyncWrappers.xOpen(opfsVfs.pointer, zDbFile,
                                               fid, openFlags, pOut);
            log("open rc =",rc,"state.sabOPView[xOpen] =",
                state.sabOPView[state.opIds.xOpen]);
            if(0!==rc){
              error("open failed with code",rc);
              return;
            }
            opfsVfs.vfsSyncWrappers.xAccess(opfsVfs.pointer, zDbFile, 0, pOut);
            rc = wasm.peek(pOut,'i32');
            if(!rc) toss("xAccess() failed to detect file.");
            rc = opfsVfs.ioSyncWrappers.xSync(sq3File.pointer, 0);
            if(rc) toss('sync failed w/ rc',rc);
            rc = opfsVfs.ioSyncWrappers.xTruncate(sq3File.pointer, 1024);
            if(rc) toss('truncate failed w/ rc',rc);
            wasm.poke(pOut,0,'i64');
            rc = opfsVfs.ioSyncWrappers.xFileSize(sq3File.pointer, pOut);
            if(rc) toss('xFileSize failed w/ rc',rc);
            log("xFileSize says:",wasm.peek(pOut, 'i64'));
            rc = opfsVfs.ioSyncWrappers.xWrite(sq3File.pointer, zDbFile, 10, 1);
            if(rc) toss("xWrite() failed!");
            const readBuf = wasm.scopedAlloc(16);
            rc = opfsVfs.ioSyncWrappers.xRead(sq3File.pointer, readBuf, 6, 2);
            wasm.poke(readBuf+6,0);
            let jRead = wasm.cstrToJs(readBuf);
            log("xRead() got:",jRead);
            if("sanity"!==jRead) toss("Unexpected xRead() value.");
            if(opfsVfs.vfsSyncWrappers.xSleep){
              log("xSleep()ing before close()ing...");
              opfsVfs.vfsSyncWrappers.xSleep(opfsVfs.pointer,2000);
              log("waking up from xSleep()");
            }
            rc = opfsVfs.ioSyncWrappers.xClose(fid);
            log("xClose rc =",rc,"sabOPView =",state.sabOPView);
            log("Deleting file:",dbFile);
            opfsVfs.vfsSyncWrappers.xDelete(opfsVfs.pointer, zDbFile, 0x1234);
            opfsVfs.vfsSyncWrappers.xAccess(opfsVfs.pointer, zDbFile, 0, pOut);
            rc = wasm.peek(pOut,'i32');
            if(rc) toss("Expecting 0 from xAccess(",dbFile,") after xDelete().");
            warn("End of OPFS sanity checks.");
          }finally{
            sq3File.dispose();
            wasm.scopedAllocPop(scope);
          }
        }/*sanityCheck()*/;

        W.onmessage = function({data}){
          //sqlite3.config.warn(vfsName,"Worker.onmessage:",data);
          switch(data.type){
            case 'opfs-unavailable':
              /* Async proxy has determined that OPFS is unavailable. There's
                 nothing more for us to do here. */
              promiseReject(new Error(data.payload.join(' ')));
              break;
            case 'opfs-async-loaded':
              /* Arrives as soon as the asyc proxy finishes loading.
                 Pass our config and shared state on to the async
                 worker. */
              delete state.vfs;
              W.postMessage({type: 'opfs-async-init', args: util.nu(state)});
              break;
            case 'opfs-async-inited': {
              /* Indicates that the async partner has received the 'init'
                 and has finished initializing, so the real work can
                 begin... */
              if(true===promiseWasRejected){
                break /* promise was already rejected via timer */;
              }
              clearTimeout(zombieTimer);
              zombieTimer = null;
              try {
                sqlite3.vfs.installVfs({
                  io: {struct: opfsVfs.ioMethods, methods: opfsVfs.ioSyncWrappers},
                  vfs: {struct: opfsVfs, methods: opfsVfs.vfsSyncWrappers}
                });
                state.sabOPView = new Int32Array(state.sabOP);
                state.sabFileBufView = new Uint8Array(state.sabIO, 0, state.fileBufferSize);
                state.sabS11nView = new Uint8Array(state.sabIO, state.sabS11nOffset, state.sabS11nSize);
                opfsVfs.initS11n();
                delete opfsVfs.initS11n;
                if(options.sanityChecks){
                  warn("Running sanity checks because of opfs-sanity-check URL arg...");
                  sanityCheck();
                }
                if(opfsUtil.thisThreadHasOPFS()){
                  opfsUtil.getRootDir().then((d)=>{
                    W.onerror = W._originalOnError;
                    delete W._originalOnError;
                    log("End of OPFS sqlite3_vfs setup.", opfsVfs);
                    promiseResolve();
                  }).catch(promiseReject);
                }else{
                  promiseResolve();
                }
              }catch(e){
                error(e);
                promiseReject(e);
              }
              break;
            }
            case 'debug':
              warn("debug message from worker:",data);
              break;
            default: {
              const errMsg = (
                "Unexpected message from the OPFS async worker: " +
                  JSON.stringify(data)
              );
              error(errMsg);
              promiseReject(new Error(errMsg));
              break;
            }
          }/*switch(data.type)*/
        }/*W.onmessage()*/;
      })/*thePromise*/;
      return thePromise;
    }/*bindVfs()*/;

    return state;
  }/*createVfsState()*/;

}/*sqlite3ApiBootstrap.initializers*/);
/*
  2022-09-18

  The author disclaims copyright to this source code.  In place of a
  legal notice, here is a blessing:

  *   May you do good and not evil.
  *   May you find forgiveness for yourself and forgive others.
  *   May you share freely, never taking more than you give.

  ***********************************************************************

  This file holds the synchronous half of an sqlite3_vfs
  implementation which proxies, in a synchronous fashion, the
  asynchronous Origin-Private FileSystem (OPFS) APIs using a second
  Worker, implemented in sqlite3-opfs-async-proxy.js.  This file is
  intended to be appended to the main sqlite3 JS deliverable somewhere
  after sqlite3-api-oo1.js.
*/
'use strict';
globalThis.sqlite3ApiBootstrap.initializers.push(function(sqlite3){
  if( !sqlite3.opfs || sqlite3.config.disable?.vfs?.opfs ){
    return;
  }
  const util = sqlite3.util,
        opfsUtil = sqlite3.opfs || sqlite3.util.toss("Missing sqlite3.opfs");
  /**
   installOpfsVfs() returns a Promise which, on success, installs an
   sqlite3_vfs named "opfs", suitable for use with all sqlite3 APIs
   which accept a VFS. It is intended to be called via
   sqlite3ApiBootstrap.initializers or an equivalent mechanism.

   The installed VFS uses the Origin-Private FileSystem API for
   all file storage. On error it is rejected with an exception
   explaining the problem. Reasons for rejection include, but are
   not limited to:

   - The counterpart Worker (see below) could not be loaded.

   - The environment does not support OPFS. That includes when
     this function is called from the main window thread.

  Significant notes and limitations:

  - The OPFS features used here are only available in dedicated Worker
    threads. This file tries to detect that case, resulting in a
    rejected Promise if those features do not seem to be available.

  - It requires the SharedArrayBuffer and Atomics classes, and the
    former is only available if the HTTP server emits the so-called
    COOP and COEP response headers. These features are required for
    proxying OPFS's synchronous API via the synchronous interface
    required by the sqlite3_vfs API.

  - This function may only be called a single time. When called, this
    function removes itself from the sqlite3 object.

  All arguments to this function are for internal/development purposes
  only. They do not constitute a public API and may change at any
  time.

  The argument may optionally be a plain object with the following
  configuration options:

  - proxyUri: name of the async proxy JS file or a synchronous function
    which, when called, returns such a name.

  - verbose (=2): an integer 0-3. 0 disables all logging, 1 enables
    logging of errors. 2 enables logging of warnings and errors. 3
    additionally enables debugging info. Logging is performed
    via the sqlite3.config.{log|warn|error}() functions.

  - sanityChecks (=false): if true, some basic sanity tests are run on
    the OPFS VFS API after it's initialized, before the returned
    Promise resolves. This is only intended for testing and
    development of the VFS, not client-side use.

  Additionaly, the (officially undocumented) 'opfs-disable' URL
  argument will disable OPFS, making this function a no-op.

  On success, the Promise resolves to the top-most sqlite3 namespace
  object. Success does not necessarily mean that it installs the VFS,
  as there are legitimate non-error reasons for OPFS not to be
  available.
*/
const installOpfsVfs = async function(options){
  options = opfsUtil.initOptions('opfs',options);
  if( !options ) return sqlite3;
  const capi = sqlite3.capi,
        state = opfsUtil.createVfsState(),
        opfsVfs = state.vfs,
        metrics = opfsVfs.metrics.counters,
        mTimeStart = opfsVfs.mTimeStart,
        mTimeEnd = opfsVfs.mTimeEnd,
        opRun = opfsVfs.opRun,
        debug = (...args)=>sqlite3.config.debug("opfs:",...args),
        warn = (...args)=>sqlite3.config.warn("opfs:",...args),
        __openFiles = opfsVfs.__openFiles;

  //debug("options:",JSON.stringify(options));
  /*
    At this point, createVfsState() has populated:

    - state: the configuration object we share with the async proxy.

    - opfsVfs: an sqlite3_vfs instance with lots of JS state attached
    to it.

    with any code common to both the "opfs" and "opfs-wl" VFSes. Now
    comes the VFS-dependent work...
  */
  return opfsVfs.bindVfs(util.nu({
    xLock: function(pFile,lockType){
      mTimeStart('xLock');
      ++metrics.xLock.count;
      const f = __openFiles[pFile];
      let rc = 0;
      /* All OPFS locks are exclusive locks. If xLock() has
         previously succeeded, do nothing except record the lock
         type. If no lock is active, have the async counterpart
         lock the file. */
      if( f.lockType ) {
        f.lockType = lockType;
      }else{
        rc = opRun('xLock', pFile, lockType);
        if( 0===rc ) f.lockType = lockType;
      }
      mTimeEnd();
      return rc;
    },
    xUnlock: function(pFile,lockType){
      mTimeStart('xUnlock');
      ++metrics.xUnlock.count;
      const f = __openFiles[pFile];
      let rc = 0;
      if( capi.SQLITE_LOCK_NONE === lockType
          && f.lockType ){
        rc = opRun('xUnlock', pFile, lockType);
      }
      if( 0===rc ) f.lockType = lockType;
      mTimeEnd();
      return rc;
    }
  }), function(sqlite3, vfs){
    /* Post-VFS-registration initialization... */
    if(sqlite3.oo1){
      const OpfsDb = function(...args){
        const opt = sqlite3.oo1.DB.dbCtorHelper.normalizeArgs(...args);
        opt.vfs = vfs.$zName;
        sqlite3.oo1.DB.dbCtorHelper.call(this, opt);
      };
      OpfsDb.prototype = Object.create(sqlite3.oo1.DB.prototype);
      sqlite3.oo1.OpfsDb = OpfsDb;
      OpfsDb.importDb = opfsUtil.importDb;
      if( true ){
        /* 2026-03-06: this was a design mis-decision and is
           inconsistent with sqlite3_open() and friends, but is
           retained against the risk of introducing regressions if
           it's removed. */
        sqlite3.oo1.DB.dbCtorHelper.setVfsPostOpenCallback(
          opfsVfs.pointer,
          function(oo1Db, sqlite3){
            /* Set a relatively high default busy-timeout handler to
               help OPFS dbs deal with multi-tab/multi-worker
               contention. */
            sqlite3.capi.sqlite3_busy_timeout(oo1Db, 10000);
          }
        );
      }
    }/*extend sqlite3.oo1*/
  })/*bindVfs()*/;
}/*installOpfsVfs()*/;
globalThis.sqlite3ApiBootstrap.initializersAsync.push(async (sqlite3)=>{
  return installOpfsVfs().catch((e)=>{
    sqlite3.config.warn("Ignoring inability to install 'opfs' sqlite3_vfs:",e);
  })
});
}/*sqlite3ApiBootstrap.initializers.push()*/);
/*
  2023-07-14

  The author disclaims copyright to this source code.  In place of a
  legal notice, here is a blessing:

  *   May you do good and not evil.
  *   May you find forgiveness for yourself and forgive others.
  *   May you share freely, never taking more than you give.

  ***********************************************************************

  This file holds a sqlite3_vfs backed by OPFS storage which uses a
  different implementation strategy than the "opfs" VFS. This one is a
  port of Roy Hashimoto's OPFS SyncAccessHandle pool:

  https://github.com/rhashimoto/wa-sqlite/blob/master/src/examples/AccessHandlePoolVFS.js

  As described at:

  https://github.com/rhashimoto/wa-sqlite/discussions/67

  with Roy's explicit permission to permit us to port his to our
  infrastructure rather than having to clean-room reverse-engineer it:

  https://sqlite.org/forum/forumpost/e140d84e71

  Primary differences from the "opfs" VFS include:

  - This one avoids the need for a sub-worker to synchronize
  communication between the synchronous C API and the
  only-partly-synchronous OPFS API.

  - It does so by opening a fixed number of OPFS files at
  library-level initialization time, obtaining SyncAccessHandles to
  each, and manipulating those handles via the synchronous sqlite3_vfs
  interface. If it cannot open them (e.g. they are already opened by
  another tab) then the VFS will not be installed.

  - Because of that, this one lacks all library-level concurrency
  support.

  - Also because of that, it does not require the SharedArrayBuffer,
  so can function without the COOP/COEP HTTP response headers.

  - It can hypothetically support Safari 16.4+, whereas the "opfs" VFS
  requires v17 due to a subworker/storage bug in 16.x which makes it
  incompatible with that VFS.

  - This VFS requires the "semi-fully-sync" FileSystemSyncAccessHandle
  (hereafter "SAH") APIs released with Chrome v108 (and all other
  major browsers released since March 2023). If that API is not
  detected, the VFS is not registered.
*/
globalThis.sqlite3ApiBootstrap.initializers.push(function(sqlite3){
  'use strict';
  if( sqlite3.config.disable?.vfs?.['opfs-sahpool'] ){
    return;
  }

  const toss = sqlite3.util.toss;
  const toss3 = sqlite3.util.toss3;
  const initPromises = Object.create(null) /* cache of (name:result) of VFS init results */;
  const capi = sqlite3.capi;
  const util = sqlite3.util;
  const wasm = sqlite3.wasm;
  // Config opts for the VFS...
  const SECTOR_SIZE = 4096;
  const HEADER_MAX_PATH_SIZE = 512;
  const HEADER_FLAGS_SIZE = 4;
  const HEADER_DIGEST_SIZE = 8;
  const HEADER_CORPUS_SIZE = HEADER_MAX_PATH_SIZE + HEADER_FLAGS_SIZE;
  const HEADER_OFFSET_FLAGS = HEADER_MAX_PATH_SIZE;
  const HEADER_OFFSET_DIGEST = HEADER_CORPUS_SIZE;
  const HEADER_OFFSET_DATA = SECTOR_SIZE;
  /* Bitmask of file types which may persist across sessions.
     SQLITE_OPEN_xyz types not listed here may be inadvertently
     left in OPFS but are treated as transient by this VFS and
     they will be cleaned up during VFS init. */
  const PERSISTENT_FILE_TYPES =
        capi.SQLITE_OPEN_MAIN_DB |
        capi.SQLITE_OPEN_MAIN_JOURNAL |
        capi.SQLITE_OPEN_SUPER_JOURNAL |
        capi.SQLITE_OPEN_WAL;
  const FLAG_COMPUTE_DIGEST_V2 = capi.SQLITE_OPEN_MEMORY
  /* Part of the fix for
     https://github.com/sqlite/sqlite-wasm/issues/97

     Summary: prior to version 3.50.0 computeDigest() always computes
     a value of [0,0] due to overflows, so it does not do anything
     useful.  Fixing it invalidates old persistent files, so we
     instead only fix it for files created or updated since the bug
     was discovered and fixed.

     This flag determines whether we use the broken legacy
     computeDigest() or the v2 variant. We only use this flag for
     newly-created/overwritten files. Pre-existing files have the
     broken digest stored in them so need to continue to use that.

     What this means, in terms of db file compatibility between
     versions:

     - DBs created with versions older than this fix (<3.50.0)
     can be read by post-fix versions. Such DBs which are written
     to in-place (not replaced) by newer versions can still be read
     by older versions, as the affected digest is only modified
     when the SAH slot is assigned to a given filename.

     - DBs created with post-fix versions will, when read by a pre-fix
     version, be seen as having a "bad digest" and will be
     unceremoniously replaced by that pre-fix version. When swapping
     back to a post-fix version, that version will see that the file
     entry is missing the FLAG_COMPUTE_DIGEST_V2 bit so will treat it
     as a legacy file.

     This flag is stored in the same memory as the various
     SQLITE_OPEN_... flags and we must be careful here to not use a
     flag bit which is otherwise relevant for the VFS.
     SQLITE_OPEN_MEMORY is handled by sqlite3_open_v2() and friends,
     not the VFS, so we'll repurpose that one.  If we take a
     currently-unused bit and it ends up, at some later point, being
     used, we would have to invalidate existing VFS files in order to
     move to another bit.  Similarly, if the SQLITE_OPEN_MEMORY bit
     were ever reassigned (which it won't be!), we'd invalidate all
     VFS-side files.
  */;

  /** Subdirectory of the VFS's space where "opaque" (randomly-named)
      files are stored. Changing this effectively invalidates the data
      stored under older names (orphaning it), so don't do that. */
  const OPAQUE_DIR_NAME = ".opaque";

  /**
     Returns short a string of random alphanumeric characters
     suitable for use as a random filename.
  */
  const getRandomName = ()=>Math.random().toString(36).slice(2);

  const textDecoder = new TextDecoder();
  const textEncoder = new TextEncoder();

  const optionDefaults = Object.assign(Object.create(null),{
    name: 'opfs-sahpool',
    directory: undefined /* derived from .name */,
    initialCapacity: 6,
    clearOnInit: false,
    /* Logging verbosity 3+ == everything, 2 == warnings+errors, 1 ==
       errors only. */
    verbosity: 2,
    forceReinitIfPreviouslyFailed: false
  });

  /** Logging routines, from most to least serious. */
  const loggers = [
    sqlite3.config.error,
    sqlite3.config.warn,
    sqlite3.config.log
  ];
  const log = sqlite3.config.log;
  const warn = sqlite3.config.warn;
  const error = sqlite3.config.error;

  /* Maps (sqlite3_vfs*) to OpfsSAHPool instances */
  const __mapVfsToPool = new Map();
  const getPoolForVfs = (pVfs)=>__mapVfsToPool.get(pVfs);
  const setPoolForVfs = (pVfs,pool)=>{
    if(pool) __mapVfsToPool.set(pVfs, pool);
    else __mapVfsToPool.delete(pVfs);
  };
  /* Maps (sqlite3_file*) to OpfsSAHPool instances */
  const __mapSqlite3File = new Map();
  const getPoolForPFile = (pFile)=>__mapSqlite3File.get(pFile);
  const setPoolForPFile = (pFile,pool)=>{
    if(pool) __mapSqlite3File.set(pFile, pool);
    else __mapSqlite3File.delete(pFile);
  };

  /**
     Impls for the sqlite3_io_methods methods. Maintenance reminder:
     members are in alphabetical order to simplify finding them.
  */
  const ioMethods = {
    xCheckReservedLock: function(pFile,pOut){
      const pool = getPoolForPFile(pFile);
      pool.log('xCheckReservedLock');
      pool.storeErr();
      wasm.poke32(pOut, 1);
      return 0;
    },
    xClose: function(pFile){
      const pool = getPoolForPFile(pFile);
      pool.storeErr();
      const file = pool.getOFileForS3File(pFile);
      if(file) {
        try{
          pool.log(`xClose ${file.path}`);
          pool.mapS3FileToOFile(pFile, false);
          file.sah.flush();
          if(file.flags & capi.SQLITE_OPEN_DELETEONCLOSE){
            pool.deletePath(file.path);
          }
        }catch(e){
          return pool.storeErr(e, capi.SQLITE_IOERR);
        }
      }
      return 0;
    },
    xDeviceCharacteristics: function(pFile){
      return capi.SQLITE_IOCAP_UNDELETABLE_WHEN_OPEN;
    },
    xFileControl: function(pFile, opId, pArg){
      return capi.SQLITE_NOTFOUND;
    },
    xFileSize: function(pFile,pSz64){
      const pool = getPoolForPFile(pFile);
      pool.log(`xFileSize`);
      const file = pool.getOFileForS3File(pFile);
      const size = file.sah.getSize() - HEADER_OFFSET_DATA;
      //log(`xFileSize ${file.path} ${size}`);
      wasm.poke64(pSz64, BigInt(size));
      return 0;
    },
    xLock: function(pFile,lockType){
      const pool = getPoolForPFile(pFile);
      pool.log(`xLock ${lockType}`);
      pool.storeErr();
      const file = pool.getOFileForS3File(pFile);
      file.lockType = lockType;
      return 0;
    },
    xRead: function(pFile,pDest,n,offset64){
      const pool = getPoolForPFile(pFile);
      pool.storeErr();
      const file = pool.getOFileForS3File(pFile);
      pool.log(`xRead ${file.path} ${n} @ ${offset64}`);
      try {
        const nRead = file.sah.read(
          wasm.heap8u().subarray(Number(pDest), Number(pDest)+n),
          {at: HEADER_OFFSET_DATA + Number(offset64)}
        );
        if(nRead < n){
          wasm.heap8u().fill(0, Number(pDest) + nRead, Number(pDest) + n);
          return capi.SQLITE_IOERR_SHORT_READ;
        }
        return 0;
      }catch(e){
        return pool.storeErr(e, capi.SQLITE_IOERR);
      }
    },
    xSectorSize: function(pFile){
      return SECTOR_SIZE;
    },
    xSync: function(pFile,flags){
      const pool = getPoolForPFile(pFile);
      pool.log(`xSync ${flags}`);
      pool.storeErr();
      const file = pool.getOFileForS3File(pFile);
      //log(`xSync ${file.path} ${flags}`);
      try{
        file.sah.flush();
        return 0;
      }catch(e){
        return pool.storeErr(e, capi.SQLITE_IOERR);
      }
    },
    xTruncate: function(pFile,sz64){
      const pool = getPoolForPFile(pFile);
      pool.log(`xTruncate ${sz64}`);
      pool.storeErr();
      const file = pool.getOFileForS3File(pFile);
      //log(`xTruncate ${file.path} ${iSize}`);
      try{
        file.sah.truncate(HEADER_OFFSET_DATA + Number(sz64));
        return 0;
      }catch(e){
        return pool.storeErr(e, capi.SQLITE_IOERR);
      }
    },
    xUnlock: function(pFile,lockType){
      const pool = getPoolForPFile(pFile);
      pool.log('xUnlock');
      const file = pool.getOFileForS3File(pFile);
      file.lockType = lockType;
      return 0;
    },
    xWrite: function(pFile,pSrc,n,offset64){
      const pool = getPoolForPFile(pFile);
      pool.storeErr();
      const file = pool.getOFileForS3File(pFile);
      pool.log(`xWrite ${file.path} ${n} ${offset64}`);
      try{
        const nBytes = file.sah.write(
          wasm.heap8u().subarray(Number(pSrc), Number(pSrc)+n),
          { at: HEADER_OFFSET_DATA + Number(offset64) }
        );
        return n===nBytes ? 0 : toss("Unknown write() failure.");
      }catch(e){
        return pool.storeErr(e, capi.SQLITE_IOERR);
      }
    }
  }/*ioMethods*/;

  const opfsIoMethods = new capi.sqlite3_io_methods();
  opfsIoMethods.$iVersion = 1;
  sqlite3.vfs.installVfs({
    io: {struct: opfsIoMethods, methods: ioMethods}
  });

  /**
     Impls for the sqlite3_vfs methods. Maintenance reminder: members
     are in alphabetical order to simplify finding them.
  */
  const vfsMethods = {
    xAccess: function(pVfs,zName,flags,pOut){
      //log(`xAccess ${wasm.cstrToJs(zName)}`);
      const pool = getPoolForVfs(pVfs);
      pool.storeErr();
      try{
        const name = pool.getPath(zName);
        wasm.poke32(pOut, pool.hasFilename(name) ? 1 : 0);
      }catch(e){
        /*ignored*/
        wasm.poke32(pOut, 0);
      }
      return 0;
    },
    xCurrentTime: function(pVfs,pOut){
      wasm.poke(pOut, 2440587.5 + (new Date().getTime()/86400000),
                'double');
      return 0;
    },
    xCurrentTimeInt64: function(pVfs,pOut){
      wasm.poke(pOut, (2440587.5 * 86400000) + new Date().getTime(),
                'i64');
      return 0;
    },
    xDelete: function(pVfs, zName, doSyncDir){
      const pool = getPoolForVfs(pVfs);
      pool.log(`xDelete ${wasm.cstrToJs(zName)}`);
      pool.storeErr();
      try{
        pool.deletePath(pool.getPath(zName));
        return 0;
      }catch(e){
        pool.storeErr(e);
        return capi.SQLITE_IOERR_DELETE;
      }
    },
    xFullPathname: function(pVfs,zName,nOut,pOut){
      //const pool = getPoolForVfs(pVfs);
      //pool.log(`xFullPathname ${wasm.cstrToJs(zName)}`);
      const i = wasm.cstrncpy(pOut, zName, nOut);
      return i<nOut ? 0 : capi.SQLITE_CANTOPEN;
    },
    xGetLastError: function(pVfs,nOut,pOut){
      const pool = getPoolForVfs(pVfs);
      const e = pool.popErr();
      pool.log(`xGetLastError ${nOut} e =`,e);
      if(e){
        const scope = wasm.scopedAllocPush();
        try{
          const [cMsg, n] = wasm.scopedAllocCString(e.message, true);
          wasm.cstrncpy(pOut, cMsg, nOut);
          if(n > nOut) wasm.poke8(wasm.ptr.add(pOut,nOut,-1), 0);
        }catch(e){
          return capi.SQLITE_NOMEM;
        }finally{
          wasm.scopedAllocPop(scope);
        }
      }
      return e ? (e.sqlite3Rc || capi.SQLITE_IOERR) : 0;
    },
    //xSleep is optionally defined below
    xOpen: function f(pVfs, zName, pFile, flags, pOutFlags){
      const pool = getPoolForVfs(pVfs);
      try{
        flags &= ~FLAG_COMPUTE_DIGEST_V2;
        pool.log(`xOpen ${wasm.cstrToJs(zName)} ${flags}`);
        // First try to open a path that already exists in the file system.
        const path = (zName && wasm.peek8(zName))
              ? pool.getPath(zName)
              : getRandomName();
        let sah = pool.getSAHForPath(path);
        if(!sah && (flags & capi.SQLITE_OPEN_CREATE)) {
          // File not found so try to create it.
          if(pool.getFileCount() < pool.getCapacity()) {
            // Choose an unassociated OPFS file from the pool.
            sah = pool.nextAvailableSAH();
            pool.setAssociatedPath(sah, path, flags);
          }else{
            // File pool is full.
            toss('SAH pool is full. Cannot create file',path);
          }
        }
        if(!sah){
          toss('file not found:',path);
        }
        // Subsequent I/O methods are only passed the sqlite3_file
        // pointer, so map the relevant info we need to that pointer.
        const file = {path, flags, sah};
        pool.mapS3FileToOFile(pFile, file);
        file.lockType = capi.SQLITE_LOCK_NONE;
        const sq3File = new capi.sqlite3_file(pFile);
        sq3File.$pMethods = opfsIoMethods.pointer;
        sq3File.dispose();
        wasm.poke32(pOutFlags, flags);
        return 0;
      }catch(e){
        pool.storeErr(e);
        return capi.SQLITE_CANTOPEN;
      }
    }/*xOpen()*/
  }/*vfsMethods*/;

  /**
     Creates, initializes, and returns an sqlite3_vfs instance for an
     OpfsSAHPool. The argument is the VFS's name (JS string).

     Throws if the VFS name is already registered or if something
     goes terribly wrong via sqlite3.vfs.installVfs().

     Maintenance reminder: the only detail about the returned object
     which is specific to any given OpfsSAHPool instance is the $zName
     member. All other state is identical.
  */
  const createOpfsVfs = function(vfsName){
    if( sqlite3.capi.sqlite3_vfs_find(vfsName)){
      toss3("VFS name is already registered:", vfsName);
    }
    const opfsVfs = new capi.sqlite3_vfs();
    /* We fetch the default VFS so that we can inherit some
       methods from it. */
    const pDVfs = capi.sqlite3_vfs_find(null);
    const dVfs = pDVfs
          ? new capi.sqlite3_vfs(pDVfs)
          : null /* dVfs will be null when sqlite3 is built with
                    SQLITE_OS_OTHER. */;
    opfsVfs.$iVersion = 2/*yes, two*/;
    opfsVfs.$szOsFile = capi.sqlite3_file.structInfo.sizeof;
    opfsVfs.$mxPathname = HEADER_MAX_PATH_SIZE;
    opfsVfs.addOnDispose(
      opfsVfs.$zName = wasm.allocCString(vfsName),
      ()=>setPoolForVfs(opfsVfs.pointer, 0)
    );

    if(dVfs){
      /* Inherit certain VFS members from the default VFS,
         if available. */
      opfsVfs.$xRandomness = dVfs.$xRandomness;
      opfsVfs.$xSleep = dVfs.$xSleep;
      dVfs.dispose();
    }
    if(!opfsVfs.$xRandomness && !vfsMethods.xRandomness){
      /* If the default VFS has no xRandomness(), add a basic JS impl... */
      vfsMethods.xRandomness = function(pVfs, nOut, pOut){
        const heap = wasm.heap8u();
        let i = 0;
        const npOut = Number(pOut);
        for(; i < nOut; ++i) heap[npOut + i] = (Math.random()*255000) & 0xFF;
        return i;
      };
    }
    if(!opfsVfs.$xSleep && !vfsMethods.xSleep){
      vfsMethods.xSleep = (pVfs,ms)=>0;
    }
    sqlite3.vfs.installVfs({
      vfs: {struct: opfsVfs, methods: vfsMethods}
    });
    return opfsVfs;
  };

  /**
     Class for managing OPFS-related state for the
     OPFS SharedAccessHandle Pool sqlite3_vfs.
  */
  class OpfsSAHPool {
    /* OPFS dir in which VFS metadata is stored. */
    vfsDir;
    /* Directory handle to this.vfsDir. */
    #dhVfsRoot;
    /* Directory handle to the subdir of this.#dhVfsRoot which holds
       the randomly-named "opaque" files. This subdir exists in the
       hope that we can eventually support client-created files in
       this.#dhVfsRoot. */
    #dhOpaque;
    /* Directory handle to this.dhVfsRoot's parent dir. Needed
       for a VFS-wipe op. */
    #dhVfsParent;
    /* Maps SAHs to their opaque file names. */
    #mapSAHToName = new Map();
    /* Maps client-side file names to SAHs. */
    #mapFilenameToSAH = new Map();
    /* Set of currently-unused SAHs. */
    #availableSAH = new Set();
    /* Maps (sqlite3_file*) to xOpen's file objects. */
    #mapS3FileToOFile_ = new Map();

    /* Maps SAH to an abstract File Object which contains
       various metadata about that handle. */

    /** Buffer used by [sg]etAssociatedPath(). */
    #apBody = new Uint8Array(HEADER_CORPUS_SIZE);
    // DataView for this.#apBody
    #dvBody;

    // associated sqlite3_vfs instance
    #cVfs;

    // Logging verbosity. See optionDefaults.verbosity.
    #verbosity;

    constructor(options = Object.create(null)){
      this.#verbosity = options.verbosity ?? optionDefaults.verbosity;
      this.vfsName = options.name || optionDefaults.name;
      this.#cVfs = createOpfsVfs(this.vfsName);
      setPoolForVfs(this.#cVfs.pointer, this);
      this.vfsDir = options.directory || ("."+this.vfsName);
      this.#dvBody =
        new DataView(this.#apBody.buffer, this.#apBody.byteOffset);
      this.isReady = this
        .reset(!!(options.clearOnInit ?? optionDefaults.clearOnInit))
        .then(()=>{
          if(this.$error) throw this.$error;
          return this.getCapacity()
            ? Promise.resolve(undefined)
            : this.addCapacity(options.initialCapacity
                               || optionDefaults.initialCapacity);
        });
    }

    #logImpl(level,...args){
      if(this.#verbosity>level) loggers[level](this.vfsName+":",...args);
    };
    log(...args){this.#logImpl(2, ...args)};
    warn(...args){this.#logImpl(1, ...args)};
    error(...args){this.#logImpl(0, ...args)};

    getVfs(){return this.#cVfs}

    /* Current pool capacity. */
    getCapacity(){return this.#mapSAHToName.size}

    /* Current number of in-use files from pool. */
    getFileCount(){return this.#mapFilenameToSAH.size}

    /* Returns an array of the names of all
       currently-opened client-specified filenames. */
    getFileNames(){
      const rc = [];
      for(const n of this.#mapFilenameToSAH.keys()) rc.push(n);
      return rc;
    }

    /**
       Adds n files to the pool's capacity. This change is
       persistent across settings. Returns a Promise which resolves
       to the new capacity.
    */
    async addCapacity(n){
      for(let i = 0; i < n; ++i){
        const name = getRandomName();
        const h = await this.#dhOpaque.getFileHandle(name, {create:true});
        const ah = await h.createSyncAccessHandle();
        this.#mapSAHToName.set(ah,name);
        this.setAssociatedPath(ah, '', 0);
        //this.#createFileObject(ah,undefined,name);
      }
      return this.getCapacity();
    }

    /**
       Reduce capacity by n, but can only reduce up to the limit
       of currently-available SAHs. Returns a Promise which resolves
       to the number of slots really removed.
    */
    async reduceCapacity(n){
      let nRm = 0;
      for(const ah of Array.from(this.#availableSAH)){
        if(nRm === n || this.getFileCount() === this.getCapacity()){
          break;
        }
        const name = this.#mapSAHToName.get(ah);
        //this.#unmapFileObject(ah);
        ah.close();
        await this.#dhOpaque.removeEntry(name);
        this.#mapSAHToName.delete(ah);
        this.#availableSAH.delete(ah);
        ++nRm;
      }
      return nRm;
    }

    /**
       Releases all currently-opened SAHs. The only legal operation
       after this is acquireAccessHandles() or (if this is called from
       pauseVfs()) either of isPaused() or unpauseVfs().
    */
    releaseAccessHandles(){
      for(const ah of this.#mapSAHToName.keys()) ah.close();
      this.#mapSAHToName.clear();
      this.#mapFilenameToSAH.clear();
      this.#availableSAH.clear();
    }

    /**
       Opens all files under this.vfsDir/this.#dhOpaque and acquires a
       SAH for each. Returns a Promise which resolves to no value but
       completes once all SAHs are acquired. If acquiring an SAH
       throws, this.$error will contain the corresponding Error
       object.

       If it throws, it releases any SAHs which it may have
       acquired before the exception was thrown, leaving the VFS in a
       well-defined but unusable state.

       If clearFiles is true, the client-stored state of each file is
       cleared when its handle is acquired, including its name, flags,
       and any data stored after the metadata block.
    */
    async acquireAccessHandles(clearFiles=false){
      const files = [];
      for await (const [name,h] of this.#dhOpaque){
        if('file'===h.kind){
          files.push([name,h]);
        }
      }
      return Promise.all(files.map(async([name,h])=>{
        try{
          const ah = await h.createSyncAccessHandle()
          this.#mapSAHToName.set(ah, name);
          if(clearFiles){
            ah.truncate(HEADER_OFFSET_DATA);
            this.setAssociatedPath(ah, '', 0);
          }else{
            const path = this.getAssociatedPath(ah);
            if(path){
              this.#mapFilenameToSAH.set(path, ah);
            }else{
              this.#availableSAH.add(ah);
            }
          }
        }catch(e){
          this.storeErr(e);
          this.releaseAccessHandles();
          throw e;
        }
      }));
    }

    /**
       Given an SAH, returns the client-specified name of
       that file by extracting it from the SAH's header.

       On error, it disassociates SAH from the pool and
       returns an empty string.
    */
    getAssociatedPath(sah){
      sah.read(this.#apBody, {at: 0});
      // Delete any unexpected files left over by previous
      // untimely errors...
      const flags = this.#dvBody.getUint32(HEADER_OFFSET_FLAGS);
      if(this.#apBody[0] &&
         ((flags & capi.SQLITE_OPEN_DELETEONCLOSE) ||
          (flags & PERSISTENT_FILE_TYPES)===0)){
        warn(`Removing file with unexpected flags ${flags.toString(16)}`,
             this.#apBody);
        this.setAssociatedPath(sah, '', 0);
        return '';
      }

      const fileDigest = new Uint32Array(HEADER_DIGEST_SIZE / 4);
      sah.read(fileDigest, {at: HEADER_OFFSET_DIGEST});
      const compDigest = this.computeDigest(this.#apBody, flags);
      //warn("getAssociatedPath() flags",'0x'+flags.toString(16), "compDigest", compDigest);
      if(fileDigest.every((v,i) => v===compDigest[i])){
        // Valid digest
        const pathBytes = this.#apBody.findIndex((v)=>0===v);
        if(0===pathBytes){
          // This file is unassociated, so truncate it to avoid
          // leaving stale db data laying around.
          sah.truncate(HEADER_OFFSET_DATA);
        }
        //warn("getAssociatedPath() flags",'0x'+flags.toString(16), "compDigest", compDigest,"pathBytes",pathBytes);
        return pathBytes
          ? textDecoder.decode(this.#apBody.subarray(0,pathBytes))
          : '';
      }else{
        // Invalid digest
        warn('Disassociating file with bad digest.');
        this.setAssociatedPath(sah, '', 0);
        return '';
      }
    }

    /**
       Stores the given client-defined path and SQLITE_OPEN_xyz flags
       into the given SAH. If path is an empty string then the file is
       disassociated from the pool but its previous name is preserved
       in the metadata.
    */
    setAssociatedPath(sah, path, flags){
      const enc = textEncoder.encodeInto(path, this.#apBody);
      if(HEADER_MAX_PATH_SIZE <= enc.written + 1/*NUL byte*/){
        toss("Path too long:",path);
      }
      if(path && flags){
        /* When creating or re-writing files, update their digest, if
           needed, to v2. We continue to use v1 for the (!path) case
           (empty files) because there's little reason not to use a
           digest of 0 for empty entries. */
        flags |= FLAG_COMPUTE_DIGEST_V2;
      }
      this.#apBody.fill(0, enc.written, HEADER_MAX_PATH_SIZE);
      this.#dvBody.setUint32(HEADER_OFFSET_FLAGS, flags);
      const digest = this.computeDigest(this.#apBody, flags);
      //console.warn("setAssociatedPath(",path,") digest",digest);
      sah.write(this.#apBody, {at: 0});
      sah.write(digest, {at: HEADER_OFFSET_DIGEST});
      sah.flush();

      if(path){
        this.#mapFilenameToSAH.set(path, sah);
        this.#availableSAH.delete(sah);
      }else{
        // This is not a persistent file, so eliminate the contents.
        sah.truncate(HEADER_OFFSET_DATA);
        this.#availableSAH.add(sah);
      }
    }

    /**
       Computes a digest for the given byte array and returns it as a
       two-element Uint32Array. This digest gets stored in the
       metadata for each file as a validation check. Changing this
       algorithm invalidates all existing databases for this VFS, so
       don't do that.

       See the docs for FLAG_COMPUTE_DIGEST_V2 for more details.
    */
    computeDigest(byteArray, fileFlags){
      if( fileFlags & FLAG_COMPUTE_DIGEST_V2 ){
        let h1 = 0xdeadbeef;
        let h2 = 0x41c6ce57;
        for(const v of byteArray){
          h1 = Math.imul(h1 ^ v, 2654435761);
          h2 = Math.imul(h2 ^ v, 104729);
        }
        return new Uint32Array([h1>>>0, h2>>>0]);
      }else{
        /* this is what the buggy legacy computation worked out to */
        return new Uint32Array([0,0]);
      }
    }

    /**
       Re-initializes the state of the SAH pool, releasing and
       re-acquiring all handles.

       See acquireAccessHandles() for the specifics of the clearFiles
       argument.
    */
    async reset(clearFiles){
      await this.isReady;
      let h = await navigator.storage.getDirectory();
      let prev, prevName;
      for(const d of this.vfsDir.split('/')){
        if(d){
          prev = h;
          h = await h.getDirectoryHandle(d,{create:true});
        }
      }
      this.#dhVfsRoot = h;
      this.#dhVfsParent = prev;
      this.#dhOpaque = await this.#dhVfsRoot.getDirectoryHandle(
        OPAQUE_DIR_NAME,{create:true}
      );
      this.releaseAccessHandles();
      return this.acquireAccessHandles(clearFiles);
    }

    /**
       Returns the pathname part of the given argument,
       which may be any of:

       - a URL object
       - A JS string representing a file name
       - Wasm C-string representing a file name

       All "../" parts and duplicate slashes are resolve/removed from
       the returned result.
    */
    getPath(arg) {
      if(wasm.isPtr(arg)) arg = wasm.cstrToJs(arg);
      return ((arg instanceof URL)
              ? arg
              : new URL(arg, 'file://localhost/')).pathname;
    }

    /**
       Removes the association of the given client-specified file
       name (JS string) from the pool. Returns true if a mapping
       is found, else false.
    */
    deletePath(path) {
      const sah = this.#mapFilenameToSAH.get(path);
      if(sah) {
        // Un-associate the name from the SAH.
        this.#mapFilenameToSAH.delete(path);
        this.setAssociatedPath(sah, '', 0);
      }
      return !!sah;
    }

    /**
       Sets e (an Error object) as this object's current error. Pass a
       falsy (or no) value to clear it. If code is truthy it is
       assumed to be an SQLITE_xxx result code, defaulting to
       SQLITE_IOERR if code is falsy.

       Returns the 2nd argument.
    */
    storeErr(e,code){
      if(e){
        e.sqlite3Rc = code || capi.SQLITE_IOERR;
        this.error(e);
      }
      this.$error = e;
      return code;
    }
    /**
       Pops this object's Error object and returns
       it (a falsy value if no error is set).
    */
    popErr(){
      const rc = this.$error;
      this.$error = undefined;
      return rc;
    }

    /**
       Returns the next available SAH without removing
       it from the set.
    */
    nextAvailableSAH(){
      const [rc] = this.#availableSAH.keys();
      return rc;
    }

    /**
       Given an (sqlite3_file*), returns the mapped
       xOpen file object.
    */
    getOFileForS3File(pFile){
      return this.#mapS3FileToOFile_.get(pFile);
    }
    /**
       Maps or unmaps (if file is falsy) the given (sqlite3_file*)
       to an xOpen file object and to this pool object.
    */
    mapS3FileToOFile(pFile,file){
      if(file){
        this.#mapS3FileToOFile_.set(pFile, file);
        setPoolForPFile(pFile, this);
      }else{
        this.#mapS3FileToOFile_.delete(pFile);
        setPoolForPFile(pFile, false);
      }
    }

    /**
       Returns true if the given client-defined file name is in this
       object's name-to-SAH map.
    */
    hasFilename(name){
      return this.#mapFilenameToSAH.has(name)
    }

    /**
       Returns the SAH associated with the given
       client-defined file name.
    */
    getSAHForPath(path){
      return this.#mapFilenameToSAH.get(path);
    }

    /**
       Removes this object's sqlite3_vfs registration and shuts down
       this object, releasing all handles, mappings, and whatnot,
       including deleting its data directory. There is currently no
       way to "revive" the object and reaquire its
       resources. Similarly, there is no recovery strategy if removal
       of any given SAH fails, so such errors are ignored by this
       function.

       This function is intended primarily for testing.

       Resolves to true if it did its job, false if the
       VFS has already been shut down.

       @see pauseVfs()
       @see unpauseVfs()
    */
    async removeVfs(){
      if(!this.#cVfs.pointer || !this.#dhOpaque) return false;
      capi.sqlite3_vfs_unregister(this.#cVfs.pointer);
      this.#cVfs.dispose();
      delete initPromises[this.vfsName];
      try{
        this.releaseAccessHandles();
        await this.#dhVfsRoot.removeEntry(OPAQUE_DIR_NAME, {recursive: true});
        this.#dhOpaque = undefined;
        await this.#dhVfsParent.removeEntry(
          this.#dhVfsRoot.name, {recursive: true}
        );
        this.#dhVfsRoot = this.#dhVfsParent = undefined;
      }catch(e){
        sqlite3.config.error(this.vfsName,"removeVfs() failed with no recovery strategy:",e);
        /*otherwise ignored - there is no recovery strategy*/
      }
      return true;
    }


    /**
       "Pauses" this VFS by unregistering it from SQLite and
       relinquishing all open SAHs, leaving the associated files
       intact. If this object is already paused, this is a
       no-op. Returns this object.

       This function throws if SQLite has any opened file handles
       hosted by this VFS, as the alternative would be to invoke
       Undefined Behavior by closing file handles out from under the
       library. Similarly, automatically closing any database handles
       opened by this VFS would invoke Undefined Behavior in
       downstream code which is holding those pointers.

       If this function throws due to open file handles then it has
       no side effects. If the OPFS API throws while closing handles
       then the VFS is left in an undefined state.

       @see isPaused()
       @see unpauseVfs()
    */
    pauseVfs(){
      if(this.#mapS3FileToOFile_.size>0){
        sqlite3.SQLite3Error.toss(
          capi.SQLITE_MISUSE, "Cannot pause VFS",
          this.vfsName,"because it has opened files."
        );
      }
      if(this.#mapSAHToName.size>0){
        capi.sqlite3_vfs_unregister(this.vfsName);
        this.releaseAccessHandles();
      }
      return this;
    }

    /**
       Returns true if this pool is currently paused else false.

       @see pauseVfs()
       @see unpauseVfs()
    */
    isPaused(){
      return 0===this.#mapSAHToName.size;
    }

    /**
       "Unpauses" this VFS, reacquiring all SAH's and (if successful)
       re-registering it with SQLite. This is a no-op if the VFS is
       not currently paused.

       The returned Promise resolves to this object. See
       acquireAccessHandles() for how it behaves if it throws due to
       SAH acquisition failure.

       @see isPaused()
       @see pauseVfs()
    */
    async unpauseVfs(){
      if(0===this.#mapSAHToName.size){
        return this.acquireAccessHandles(false).
          then(()=>capi.sqlite3_vfs_register(this.#cVfs, 0),this);
      }
      return this;
    }

    //! Documented elsewhere in this file.
    exportFile(name){
      const sah = this.#mapFilenameToSAH.get(name) || toss("File not found:",name);
      const n = sah.getSize() - HEADER_OFFSET_DATA;
      const b = new Uint8Array(n>0 ? n : 0);
      if(n>0){
        const nRead = sah.read(b, {at: HEADER_OFFSET_DATA});
        if(nRead != n){
          toss("Expected to read "+n+" bytes but read "+nRead+".");
        }
      }
      return b;
    }

    //! Impl for importDb() when its 2nd arg is a function.
    async importDbChunked(name, callback){
      const sah = this.#mapFilenameToSAH.get(name)
            || this.nextAvailableSAH()
            || toss("No available handles to import to.");
      sah.truncate(0);
      let nWrote = 0, chunk, checkedHeader = false, err = false;
      try{
        while( undefined !== (chunk = await callback()) ){
          if(chunk instanceof ArrayBuffer) chunk = new Uint8Array(chunk);
          if( !checkedHeader && 0===nWrote && chunk.byteLength>=15 ){
            util.affirmDbHeader(chunk);
            checkedHeader = true;
          }
          sah.write(chunk, {at:  HEADER_OFFSET_DATA + nWrote});
          nWrote += chunk.byteLength;
        }
        if( nWrote < 512 || 0!==nWrote % 512 ){
          toss("Input size",nWrote,"is not correct for an SQLite database.");
        }
        if( !checkedHeader ){
          const header = new Uint8Array(20);
          sah.read( header, {at: 0} );
          util.affirmDbHeader( header );
        }
        sah.write(new Uint8Array([1,1]), {
          at: HEADER_OFFSET_DATA + 18
        }/*force db out of WAL mode*/);
      }catch(e){
        this.setAssociatedPath(sah, '', 0);
        throw e;
      }
      this.setAssociatedPath(sah, name, capi.SQLITE_OPEN_MAIN_DB);
      return nWrote;
    }

    //! Documented elsewhere in this file.
    importDb(name, bytes){
      if( bytes instanceof ArrayBuffer ) bytes = new Uint8Array(bytes);
      else if( bytes instanceof Function ) return this.importDbChunked(name, bytes);
      const sah = this.#mapFilenameToSAH.get(name)
            || this.nextAvailableSAH()
            || toss("No available handles to import to.");
      const n = bytes.byteLength;
      if(n<512 || n%512!=0){
        toss("Byte array size is invalid for an SQLite db.");
      }
      const header = "SQLite format 3";
      for(let i = 0; i < header.length; ++i){
        if( header.charCodeAt(i) !== bytes[i] ){
          toss("Input does not contain an SQLite database header.");
        }
      }
      const nWrote = sah.write(bytes, {at: HEADER_OFFSET_DATA});
      if(nWrote != n){
        this.setAssociatedPath(sah, '', 0);
        toss("Expected to write "+n+" bytes but wrote "+nWrote+".");
      }else{
        sah.write(new Uint8Array([1,1]), {at: HEADER_OFFSET_DATA+18}
                   /* force db out of WAL mode */);
        this.setAssociatedPath(sah, name, capi.SQLITE_OPEN_MAIN_DB);
      }
      return nWrote;
    }

  }/*class OpfsSAHPool*/;


  /**
     A OpfsSAHPoolUtil instance is exposed to clients in order to
     manipulate an OpfsSAHPool object without directly exposing that
     object and allowing for some semantic changes compared to that
     class.

     Class docs are in the client-level docs for
     installOpfsSAHPoolVfs().
  */
  class OpfsSAHPoolUtil {
    /* This object's associated OpfsSAHPool. */
    #p;

    constructor(sahPool){
      this.#p = sahPool;
      this.vfsName = sahPool.vfsName;
    }

    async addCapacity(n){ return this.#p.addCapacity(n) }

    async reduceCapacity(n){ return this.#p.reduceCapacity(n) }

    getCapacity(){ return this.#p.getCapacity(this.#p) }

    getFileCount(){ return this.#p.getFileCount() }
    getFileNames(){ return this.#p.getFileNames() }

    async reserveMinimumCapacity(min){
      const c = this.#p.getCapacity();
      return (c < min) ? this.#p.addCapacity(min - c) : c;
    }

    exportFile(name){ return this.#p.exportFile(name) }

    importDb(name, bytes){ return this.#p.importDb(name,bytes) }

    async wipeFiles(){ return this.#p.reset(true) }

    unlink(filename){ return this.#p.deletePath(filename) }

    async removeVfs(){ return this.#p.removeVfs() }

    pauseVfs(){ this.#p.pauseVfs(); return this; }
    async unpauseVfs(){ return this.#p.unpauseVfs().then(()=>this); }
    isPaused(){ return this.#p.isPaused() }

  }/* class OpfsSAHPoolUtil */;

  /**
     Returns a resolved Promise if the current environment
     has a "fully-sync" SAH impl, else a rejected Promise.
  */
  const apiVersionCheck = async ()=>{
    const dh = await navigator.storage.getDirectory();
    const fn = '.opfs-sahpool-sync-check-'+getRandomName();
    const fh = await dh.getFileHandle(fn, { create: true });
    const ah = await fh.createSyncAccessHandle();
    const close = ah.close();
    await close;
    await dh.removeEntry(fn);
    if(close?.then){
      toss("The local OPFS API is too old for opfs-sahpool:",
           "it has an async FileSystemSyncAccessHandle.close() method.");
    }
    return true;
  };

  /**
     installOpfsSAHPoolVfs() asynchronously initializes the OPFS
     SyncAccessHandle (a.k.a. SAH) Pool VFS. It returns a Promise which
     either resolves to a utility object described below or rejects with
     an Error value.

     Initialization of this VFS is not automatic because its
     registration requires that it lock all resources it
     will potentially use, even if client code does not want
     to use them. That, in turn, can lead to locking errors
     when, for example, one page in a given origin has loaded
     this VFS but does not use it, then another page in that
     origin tries to use the VFS. If the VFS were automatically
     registered, the second page would fail to load the VFS
     due to OPFS locking errors.

     If this function is called more than once with a given "name"
     option (see below), it will return the same Promise. Calls for
     different names will return different Promises which resolve to
     independent objects and refer to different VFS registrations.

     On success, the resulting Promise resolves to a utility object
     which can be used to query and manipulate the pool. Its API is
     described at the end of these docs.

     This function accepts an options object to configure certain
     parts but it is only acknowledged for the very first call for
     each distinct name and ignored for all subsequent calls with that
     same name.

     The options, in alphabetical order:

     - `clearOnInit`: (default=false) if truthy, contents and filename
     mapping are removed from each SAH it is acquired during
     initialization of the VFS, leaving the VFS's storage in a pristine
     state. Use this only for databases which need not survive a page
     reload.

     - `initialCapacity`: (default=6) Specifies the default capacity of
     the VFS. This should not be set unduly high because the VFS has
     to open (and keep open) a file for each entry in the pool. This
     setting only has an effect when the pool is initially empty. It
     does not have any effect if a pool already exists.

     - `directory`: (default="."+`name`) Specifies the OPFS directory
     name in which to store metadata for the `"opfs-sahpool"`
     sqlite3_vfs.  Only one instance of this VFS can be installed per
     JavaScript engine, and any two engines with the same storage
     directory name will collide with each other, leading to locking
     errors and the inability to register the VFS in the second and
     subsequent engine. Using a different directory name for each
     application enables different engines in the same HTTP origin to
     co-exist, but their data are invisible to each other. Changing
     this name will effectively orphan any databases stored under
     previous names. The default is unspecified but descriptive.  This
     option may contain multiple path elements, e.g. "foo/bar/baz",
     and they are created automatically.  In practice there should be
     no driving need to change this. ACHTUNG: all files in this
     directory are assumed to be managed by the VFS. Do not place
     other files in that directory, as they may be deleted or
     otherwise modified by the VFS.

     - `name`: (default="opfs-sahpool") sets the name to register this
     VFS under. Normally this should not be changed, but it is
     possible to register this VFS under multiple names so long as
     each has its own separate directory to work from. The storage for
     each is invisible to all others. The name must be a string
     compatible with `sqlite3_vfs_register()` and friends and suitable
     for use in URI-style database file names.

     Achtung: if a custom `name` is provided, a custom `directory`
     must also be provided if any other instance is registered with
     the default directory. If no directory is explicitly provided
     then a directory name is synthesized from the `name` option.


     - `forceReinitIfPreviouslyFailed`: (default=`false`) Is a fallback option
     to assist in working around certain flaky environments which may
     mysteriously fail to permit access to OPFS sync access handles on
     an initial attempt but permit it on a second attemp. This option
     should never be used but is provided for those who choose to
     throw caution to the wind and trust such environments. If this
     option is truthy _and_ the previous attempt to initialize this
     VFS with the same `name` failed, the VFS will attempt to
     initialize a second time instead of returning the cached
     failure. See discussion at:
     <https://github.com/sqlite/sqlite-wasm/issues/79>


     Peculiarities of this VFS vis a vis other SQLite VFSes:

     - Paths given to it _must_ be absolute. Relative paths will not
     be properly recognized. This is arguably a bug but correcting it
     requires some hoop-jumping in routines which have no business
     doing such tricks. (2026-01-19 (2.5 years later): the specifics
     are lost to history, but this was a side effect of xOpen()
     receiving an immutable C-string filename, to which no implicit
     "/" can be prefixed without causing a discrepancy between what
     the user provided and what the VFS stores. Its conceivable that
     that quirk could be glossed over in xFullPathname(), but
     regressions when doing so cannot be ruled out, so there are no
     current plans to change this behavior.)

     - It is possible to install multiple instances under different
     names, each sandboxed from one another inside their own private
     directory.  This feature exists primarily as a way for disparate
     applications within a given HTTP origin to use this VFS without
     introducing locking issues between them.


     The API for the utility object passed on by this function's
     Promise, in alphabetical order...

     - [async] number addCapacity(n)

     Adds `n` entries to the current pool. This change is persistent
     across sessions so should not be called automatically at each app
     startup (but see `reserveMinimumCapacity()`). Its returned Promise
     resolves to the new capacity.  Because this operation is necessarily
     asynchronous, the C-level VFS API cannot call this on its own as
     needed.

     - byteArray exportFile(name)

     Synchronously reads the contents of the given file into a Uint8Array
     and returns it. This will throw if the given name is not currently
     in active use or on I/O error. Note that the given name is _not_
     visible directly in OPFS (or, if it is, it's not from this VFS).

     - number getCapacity()

     Returns the number of files currently contained
     in the SAH pool. The default capacity is only large enough for one
     or two databases and their associated temp files.

     - number getFileCount()

     Returns the number of files from the pool currently allocated to
     slots. This is not the same as the files being "opened".

     - array getFileNames()

     Returns an array of the names of the files currently allocated to
     slots. This list is the same length as getFileCount().

     - void importDb(name, bytes)

     Imports the contents of an SQLite database, provided as a byte
     array or ArrayBuffer, under the given name, overwriting any
     existing content. Throws if the pool has no available file slots,
     on I/O error, or if the input does not appear to be a
     database. In the latter case, only a cursory examination is made.
     Results are undefined if the given db name refers to an opened
     db.  Note that this routine is _only_ for importing database
     files, not arbitrary files, the reason being that this VFS will
     automatically clean up any non-database files so importing them
     is pointless.

     If passed a function for its second argument, its behavior
     changes to asynchronous and it imports its data in chunks fed to
     it by the given callback function. It calls the callback (which
     may be async) repeatedly, expecting either a Uint8Array or
     ArrayBuffer (to denote new input) or undefined (to denote
     EOF). For so long as the callback continues to return
     non-undefined, it will append incoming data to the given
     VFS-hosted database file. The result of the resolved Promise when
     called this way is the size of the resulting database.

     On success this routine rewrites the database header bytes in the
     output file (not the input array) to force disabling of WAL mode.

     On a write error, the handle is removed from the pool and made
     available for re-use.

     - [async] number reduceCapacity(n)

     Removes up to `n` entries from the pool, with the caveat that it can
     only remove currently-unused entries. It returns a Promise which
     resolves to the number of entries actually removed.

     - [async] boolean removeVfs()

     Unregisters the opfs-sahpool VFS and removes its directory from OPFS
     (which means that _all client content_ is removed). After calling
     this, the VFS may no longer be used and there is no way to re-add it
     aside from reloading the current JavaScript context.

     Results are undefined if a database is currently in use with this
     VFS.

     The returned Promise resolves to true if it performed the removal
     and false if the VFS was not installed.

     If the VFS has a multi-level directory, e.g. "/foo/bar/baz", _only_
     the bottom-most directory is removed because this VFS cannot know for
     certain whether the higher-level directories contain data which
     should be removed.

     - [async] number reserveMinimumCapacity(min)

     If the current capacity is less than `min`, the capacity is
     increased to `min`, else this returns with no side effects. The
     resulting Promise resolves to the new capacity.

     - boolean unlink(filename)

     If a virtual file exists with the given name, disassociates it from
     the pool and returns true, else returns false without side
     effects. Results are undefined if the file is currently in active
     use.

     - string vfsName

     The SQLite VFS name under which this pool's VFS is registered.

     - [async] void wipeFiles()

     Clears all client-defined state of all SAHs and makes all of them
     available for re-use by the pool. Results are undefined if any such
     handles are currently in use, e.g. by an sqlite3 db.

     APIs specific to the "pause" capability (added in version 3.49):

     Summary: "pausing" the VFS disassociates it from SQLite and
     relinquishes its SAHs so that they may be opened by another
     instance of this VFS (running in a separate tab/page or Worker).
     "Unpausing" it takes back control, if able.

     - pauseVfs()

     "Pauses" this VFS by unregistering it from SQLite and
     relinquishing all open SAHs, leaving the associated files intact.
     This enables pages/tabs to coordinate semi-concurrent usage of
     this VFS.  If this object is already paused, this is a
     no-op. Returns this object. Throws if SQLite has any opened file
     handles hosted by this VFS. If this function throws due to open
     file handles then it has no side effects. If the OPFS API throws
     while closing handles then the VFS is left in an undefined state.

     - isPaused()

     Returns true if this VFS is paused, else false.

     - [async] unpauseVfs()

     Restores the VFS to an active state after having called
     pauseVfs() on it.  This is a no-op if the VFS is not paused. The
     returned Promise resolves to this object on success. A rejected
     Promise means there was a problem reacquiring the SAH handles
     (possibly because they're in use by another instance or have
     since been removed). Generically speaking, there is no recovery
     strategy for that type of error, but if the problem is simply
     that the OPFS files are locked, then a later attempt to unpause
     it, made after the concurrent instance releases the SAHs, may
     recover from the situation.
  */
  sqlite3.installOpfsSAHPoolVfs = async function(options=Object.create(null)){
    options = Object.assign(Object.create(null), optionDefaults, (options||{}));
    const vfsName = options.name;
    if(options.$testThrowPhase1){
      throw options.$testThrowPhase1;
    }
    if(initPromises[vfsName]){
      try {
        const p = await initPromises[vfsName];
        //log("installOpfsSAHPoolVfs() returning cached result",options,vfsName,p);
        return p;
      }catch(e){
        //log("installOpfsSAHPoolVfs() got cached failure",options,vfsName,e);
        if( options.forceReinitIfPreviouslyFailed ){
          delete initPromises[vfsName];
          /* Fall through and try again. */
        }else{
          throw e;
        }
      }
    }
    if(!globalThis.FileSystemHandle ||
       !globalThis.FileSystemDirectoryHandle ||
       !globalThis.FileSystemFileHandle ||
       !globalThis.FileSystemFileHandle.prototype.createSyncAccessHandle ||
       !navigator?.storage?.getDirectory){
      return (initPromises[vfsName] = Promise.reject(new Error("Missing required OPFS APIs.")));
    }

    /**
       Maintenance reminder: the order of ASYNC ops in this function
       is significant. We need to have them all chained at the very
       end in order to be able to catch a race condition where
       installOpfsSAHPoolVfs() is called twice in rapid succession,
       e.g.:

       installOpfsSAHPoolVfs().then(console.warn.bind(console));
       installOpfsSAHPoolVfs().then(console.warn.bind(console));

       If the timing of the async calls is not "just right" then that
       second call can end up triggering the init a second time and chaos
       ensues.
    */
    return initPromises[vfsName] = apiVersionCheck().then(async function(){
      if(options.$testThrowPhase2){
        throw options.$testThrowPhase2;
      }
      const thePool = new OpfsSAHPool(options);
      return thePool.isReady.then(async()=>{
        /** The poolUtil object will be the result of the
            resolved Promise. */
        const poolUtil = new OpfsSAHPoolUtil(thePool);
        if(sqlite3.oo1){
          const oo1 = sqlite3.oo1;
          const theVfs = thePool.getVfs();
          const OpfsSAHPoolDb = function(...args){
            const opt = oo1.DB.dbCtorHelper.normalizeArgs(...args);
            opt.vfs = theVfs.$zName;
            oo1.DB.dbCtorHelper.call(this, opt);
          };
          OpfsSAHPoolDb.prototype = Object.create(oo1.DB.prototype);
          poolUtil.OpfsSAHPoolDb = OpfsSAHPoolDb;
        }/*extend sqlite3.oo1*/
        thePool.log("VFS initialized.");
        return poolUtil;
      }).catch(async (e)=>{
        await thePool.removeVfs().catch(()=>{});
        throw e;
      });
    }).catch((err)=>{
      //error("rejecting promise:",err);
      return initPromises[vfsName] = Promise.reject(err);
    });
  }/*installOpfsSAHPoolVfs()*/;
}/*sqlite3ApiBootstrap.initializers*/);
/*
  2026-02-20

  The author disclaims copyright to this source code.  In place of a
  legal notice, here is a blessing:

  *   May you do good and not evil.
  *   May you find forgiveness for yourself and forgive others.
  *   May you share freely, never taking more than you give.

  ***********************************************************************

  This file is a reimplementation of the "opfs" VFS (as distinct from
  "opfs-sahpool") which uses WebLocks for locking instead of a bespoke
  Atomics.wait()/notify() protocol. This file holds the "synchronous
  half" of the VFS, whereas it shares the "asynchronous half" with the
  "opfs" VFS.

  Testing has failed to show any genuine functional difference between
  these VFSes other than "opfs-wl" being able to dole out xLock()
  requests in a strictly FIFO manner by virtue of WebLocks being
  globally managed by the browser. This tends to lead to, but does not
  guaranty, fairer distribution of locks. Differences are unlikely to
  be noticed except, perhaps, under very high contention.

  This file is intended to be appended to the main sqlite3 JS
  deliverable somewhere after opfs-common-shared.c-pp.js.
*/
'use strict';
globalThis.sqlite3ApiBootstrap.initializers.push(function(sqlite3){
  if( !sqlite3.opfs || sqlite3.config.disable?.vfs?.['opfs-wl'] ){
    return;
  }
  const util = sqlite3.util,
        toss  = sqlite3.util.toss;
  const opfsUtil = sqlite3.opfs;
  const vfsName = 'opfs-wl';
/**
   installOpfsWlVfs() returns a Promise which, on success, installs an
   sqlite3_vfs named "opfs-wl", suitable for use with all sqlite3 APIs
   which accept a VFS. It is intended to be called via
   sqlite3ApiBootstrap.initializers or an equivalent mechanism.

   This VFS is essentially identical to the "opfs" VFS but uses
   WebLocks for its xLock() and xUnlock() implementations.

   Quirks specific to this VFS:

   - The (officially undocumented) 'opfs-wl-disable' URL
   argument will disable OPFS, making this function a no-op.

   Aside from locking differences in the VFSes, this function
   otherwise behaves the same as
   sqlite3-vfs-opfs.c-pp.js:installOpfsVfs().
*/
const installOpfsWlVfs = async function(options){
  options = opfsUtil.initOptions(vfsName,options);
  if( !options ) return sqlite3;
  const capi = sqlite3.capi,
        state = opfsUtil.createVfsState(),
        opfsVfs = state.vfs,
        metrics = opfsVfs.metrics.counters,
        mTimeStart = opfsVfs.mTimeStart,
        mTimeEnd = opfsVfs.mTimeEnd,
        opRun = opfsVfs.opRun,
        debug = (...args)=>sqlite3.config.debug(vfsName+":",...args),
        warn = (...args)=>sqlite3.config.warn(vfsName+":",...args),
        __openFiles = opfsVfs.__openFiles;

  //debug("state",JSON.stringify(options));
  /*
    At this point, createVfsState() has populated:

    - state: the configuration object we share with the async proxy.

    - opfsVfs: an sqlite3_vfs instance with lots of JS state attached
    to it.

    with any code common to both the "opfs" and "opfs-wl" VFSes. Now
    comes the VFS-dependent work...
  */
  return opfsVfs.bindVfs(util.nu({
    xLock: function(pFile,lockType){
      mTimeStart('xLock');
      //debug("xLock()...");
      const f = __openFiles[pFile];
      const rc = opRun('xLock', pFile, lockType);
      if( !rc ) f.lockType = lockType;
      mTimeEnd();
      return rc;
    },
    xUnlock: function(pFile,lockType){
      mTimeStart('xUnlock');
      const f = __openFiles[pFile];
      const rc = opRun('xUnlock', pFile, lockType);
      if( !rc ) f.lockType = lockType;
      mTimeEnd();
      return rc;
    }
  }), function(sqlite3, vfs){
    /* Post-VFS-registration initialization... */
    if(sqlite3.oo1){
      const OpfsWlDb = function(...args){
        const opt = sqlite3.oo1.DB.dbCtorHelper.normalizeArgs(...args);
        opt.vfs = vfs.$zName;
        sqlite3.oo1.DB.dbCtorHelper.call(this, opt);
      };
      OpfsWlDb.prototype = Object.create(sqlite3.oo1.DB.prototype);
      sqlite3.oo1.OpfsWlDb = OpfsWlDb;
      OpfsWlDb.importDb = opfsUtil.importDb;
      /* The "opfs" VFS variant adds a
         oo1.DB.dbCtorHelper.setVfsPostOpenCallback() callback to set
         a high busy_timeout. That was a design mis-decision and is
         inconsistent with sqlite3_open() and friends, but is retained
         against the risk of introducing regressions if it's removed.
         This variant does not repeat that mistake.
      */
    }
  })/*bindVfs()*/;
}/*installOpfsWlVfs()*/;
globalThis.sqlite3ApiBootstrap.initializersAsync.push(async (sqlite3)=>{
  return installOpfsWlVfs().catch((e)=>{
    sqlite3.config.warn("Ignoring inability to install the",vfsName,"sqlite3_vfs:",e);
  });
});
}/*sqlite3ApiBootstrap.initializers.push()*/);
/*
  2022-07-22

  The author disclaims copyright to this source code.  In place of a
  legal notice, here is a blessing:

  *   May you do good and not evil.
  *   May you find forgiveness for yourself and forgive others.
  *   May you share freely, never taking more than you give.

  ***********************************************************************

  This file is the tail end of the sqlite3-api.js constellation,
  closing the function scope opened by post-js-header.js.

  In terms of amalgamation code placement, this file is appended
  immediately after the final sqlite3-api-*.js piece. Those files
  cooperate to prepare sqlite3ApiBootstrap() and this file calls it.
  It is run within a context which gives it access to Emscripten's
  Module object, after sqlite3.wasm is loaded but before
  sqlite3ApiBootstrap() has been called.

  Because this code resides (after building) inside the function
  installed by post-js-header.js, it has access to state set up by
  pre-js.c-pp.js and friends.
*/
try{
  /* We are in the closing block of Module.runSQLite3PostLoadInit(), so
     its arguments are visible here. */

  /* Config options for sqlite3ApiBootstrap(). */
  const bootstrapConfig = Object.assign(
    Object.create(null),
    /** The WASM-environment-dependent configuration for sqlite3ApiBootstrap() */
    {
      memory: ('undefined'!==typeof wasmMemory)
        ? wasmMemory
        : EmscriptenModule['wasmMemory'],
      exports: ('undefined'!==typeof wasmExports)
        ? wasmExports /* emscripten >=3.1.44 */
        : (Object.prototype.hasOwnProperty.call(EmscriptenModule,'wasmExports')
           ? EmscriptenModule['wasmExports']
           : EmscriptenModule['asm']/* emscripten <=3.1.43 */)
    },
    globalThis.sqlite3ApiBootstrap.defaultConfig, // default options
    globalThis.sqlite3ApiConfig || {} // optional client-provided options
  );

  sqlite3InitScriptInfo.debugModule("Bootstrapping lib config", bootstrapConfig);

  /**
     For purposes of the Emscripten build, call sqlite3ApiBootstrap().
     Ideally clients should be able to inject their own config here,
     but that's not practical in this particular build constellation
     because of the order everything happens in.  Clients may either
     define globalThis.sqlite3ApiConfig or modify
     globalThis.sqlite3ApiBootstrap.defaultConfig to tweak the default
     configuration used by a no-args call to sqlite3ApiBootstrap(),
     but must have first loaded their WASM module in order to be able
     to provide the necessary configuration state.
  */
  const p = globalThis.sqlite3ApiBootstrap(bootstrapConfig);
  delete globalThis.sqlite3ApiBootstrap;
  return p /* the eventual result of globalThis.sqlite3InitModule() */;
}catch(e){
  console.error("sqlite3ApiBootstrap() error:",e);
  throw e;
}

//console.warn("This is the end of the Module.runSQLite3PostLoadInit handler.");
}/*Module.runSQLite3PostLoadInit(...)*/;
//console.warn("This is the end of the setup of the (pending) Module.runSQLite3PostLoadInit");
// end include: /src/dist/.build/sqlite-src-3530300/ext/wasm/bld/post-js.esm.js

// include: postamble_modularize.js
// In MODULARIZE mode we wrap the generated code in a factory function
// and return either the Module itself, or a promise of the module.
//
// We assign to the `moduleRtn` global here and configure closure to see
// this as and extern so it won't get minified.

moduleRtn = readyPromise;

// Assertion for attempting to access module properties on the incoming
// moduleArg.  In the past we used this object as the prototype of the module
// and assigned properties to it, but now we return a distinct object.  This
// keeps the instance private until it is ready (i.e the promise has been
// resolved).
for (const prop of Object.keys(Module)) {
  if (!(prop in moduleArg)) {
    Object.defineProperty(moduleArg, prop, {
      configurable: true,
      get() {
        abort(`Access to module property ('${prop}') is no longer possible via the module constructor argument; Instead, use the result of the module constructor.`)
      }
    });
  }
}
// end include: postamble_modularize.js



  return moduleRtn;
}
);
})();

/* ^^^^ ACHTUNG: blank line at the start is necessary because
   Emscripten will not add a newline in some cases and we need
   a blank line for a sed-based kludge for the ES6 build.

   extern-post-js.js must be appended to the resulting sqlite3.js
   file. It gets its name from being used as the value for the
   --extern-post-js=... Emscripten flag. This code, unlike most of the
   associated JS code, runs outside of the Emscripten-generated module
   init scope, in the current global scope.

   At the time this is run, the global-scope sqlite3InitModule
   function will have just been defined.
*/
const toExportForESM =
(function(){
  //console.warn("this is extern-post-js");
  /**
     In order to hide the sqlite3InitModule()'s resulting
     Emscripten module from downstream clients (and simplify our
     documentation by being able to elide those details), we hide that
     function and expose a hand-written sqlite3InitModule() to return
     the sqlite3 object (most of the time).
  */
  const originalInit = sqlite3InitModule;
  if(!originalInit){
    throw new Error("Expecting sqlite3InitModule to be defined by the Emscripten build.");
  }
  /**
     We need to add some state which our custom Module.locateFile()
     can see, but an Emscripten limitation currently prevents us from
     attaching it to the sqlite3InitModule function object:

     https://github.com/emscripten-core/emscripten/issues/18071

     The only(?) current workaround is to temporarily stash this state
     into the global scope and delete it when sqlite3InitModule()
     is called.
  */
  const sIMS = globalThis.sqlite3InitModuleState = Object.assign(Object.create(null),{
    moduleScript: globalThis?.document?.currentScript,
    isWorker: ('undefined' !== typeof WorkerGlobalScope),
    location: globalThis.location,
    urlParams:  globalThis?.location?.href
      ? new URL(globalThis.location.href).searchParams
      : new URLSearchParams(),
    /*
      It is literally impossible to reliably get the name of _this_ script
      at runtime, so impossible to reliably derive X.wasm from script name
      X.js. (This is apparently why Emscripten hard-codes the name of the
      wasm file into their output.)  Thus we need, at build-time, to set
      the name of the WASM file which our custom instantiateWasm() should to
      load. The build process populates this.

      Module.instantiateWasm() is found in pre-js.c-pp.js.
    */
    wasmFilename: 'sqlite3.wasm' /* replaced by the build process */
  });
  sIMS.debugModule =
    sIMS.urlParams.has('sqlite3.debugModule')
    ? (...args)=>console.warn('sqlite3.debugModule:',...args)
    : ()=>{};

  if(sIMS.urlParams.has('sqlite3.dir')){
    sIMS.sqlite3Dir = sIMS.urlParams.get('sqlite3.dir') +'/';
  }else if(sIMS.moduleScript){
    const li = sIMS.moduleScript.src.split('/');
    li.pop();
    sIMS.sqlite3Dir = li.join('/') + '/';
  }

  const sIM = globalThis.sqlite3InitModule = function ff(...args){
    //console.warn("Using replaced sqlite3InitModule()",globalThis.location);
    sIMS.emscriptenLocateFile = args[0]?.locateFile /* see pre-js.c-pp.js [tag:locateFile] */;
    sIMS.emscriptenInstantiateWasm = args[0]?.instantiateWasm /* see pre-js.c-pp.js [tag:locateFile] */;
    return originalInit(...args).then((EmscriptenModule)=>{
      sIMS.debugModule("sqlite3InitModule() sIMS =",sIMS);
      sIMS.debugModule("sqlite3InitModule() EmscriptenModule =",EmscriptenModule);
      const s = EmscriptenModule.runSQLite3PostLoadInit(
        sIMS,
        EmscriptenModule /* see post-js-header/footer.js */,
        !!ff.__isUnderTest
      );
      sIMS.debugModule("sqlite3InitModule() sqlite3 =",s);
      //const rv = s.asyncPostInit();
      //delete s.asyncPostInit;
      return s;
    }).catch((e)=>{
      console.error("Exception loading sqlite3 module:",e);
      throw e;
    });
  };
  sIM.ready = originalInit.ready;

  if(sIMS.moduleScript){
    let src = sIMS.moduleScript.src.split('/');
    src.pop();
    sIMS.scriptDir = src.join('/') + '/';
  }
  sIMS.debugModule('extern-post-js.c-pp.js sqlite3InitModuleState =',sIMS);
  return sIM;
})();
sqlite3InitModule = toExportForESM;
export default sqlite3InitModule;
