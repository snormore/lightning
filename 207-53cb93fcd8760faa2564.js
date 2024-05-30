(window.webpackJsonp=window.webpackJsonp||[]).push([[207],{"1GJ6":function(e,t,n){"use strict";n.r(t),n.d(t,"setupMode",(function(){return xe}));var r,i,o,a,s,u,c,d,f,l,g,h,p,m,v,b,k,C,_,w=function(){function e(e){var t=this;this._defaults=e,this._worker=null,this._idleCheckInterval=setInterval((function(){return t._checkIfIdle()}),3e4),this._lastUsedTime=0,this._configChangeListener=this._defaults.onDidChange((function(){return t._stopWorker()}))}return e.prototype._stopWorker=function(){this._worker&&(this._worker.dispose(),this._worker=null),this._client=null},e.prototype.dispose=function(){clearInterval(this._idleCheckInterval),this._configChangeListener.dispose(),this._stopWorker()},e.prototype._checkIfIdle=function(){this._worker&&(Date.now()-this._lastUsedTime>12e4&&this._stopWorker())},e.prototype._getClient=function(){return this._lastUsedTime=Date.now(),this._client||(this._worker=monaco.editor.createWebWorker({moduleId:"vs/language/json/jsonWorker",label:this._defaults.languageId,createData:{languageSettings:this._defaults.diagnosticsOptions,languageId:this._defaults.languageId,enableSchemaRequest:this._defaults.diagnosticsOptions.enableSchemaRequest}}),this._client=this._worker.getProxy()),this._client},e.prototype.getLanguageServiceWorker=function(){for(var e,t=this,n=[],r=0;r<arguments.length;r++)n[r]=arguments[r];return this._getClient().then((function(t){e=t})).then((function(e){return t._worker.withSyncedResources(n)})).then((function(t){return e}))},e}();!function(e){e.create=function(e,t){return{line:e,character:t}},e.is=function(e){var t=e;return Q.objectLiteral(t)&&Q.number(t.line)&&Q.number(t.character)}}(r||(r={})),function(e){e.create=function(e,t,n,i){if(Q.number(e)&&Q.number(t)&&Q.number(n)&&Q.number(i))return{start:r.create(e,t),end:r.create(n,i)};if(r.is(e)&&r.is(t))return{start:e,end:t};throw new Error("Range#create called with invalid arguments["+e+", "+t+", "+n+", "+i+"]")},e.is=function(e){var t=e;return Q.objectLiteral(t)&&r.is(t.start)&&r.is(t.end)}}(i||(i={})),function(e){e.create=function(e,t){return{uri:e,range:t}},e.is=function(e){var t=e;return Q.defined(t)&&i.is(t.range)&&(Q.string(t.uri)||Q.undefined(t.uri))}}(o||(o={})),function(e){e.create=function(e,t,n,r){return{targetUri:e,targetRange:t,targetSelectionRange:n,originSelectionRange:r}},e.is=function(e){var t=e;return Q.defined(t)&&i.is(t.targetRange)&&Q.string(t.targetUri)&&(i.is(t.targetSelectionRange)||Q.undefined(t.targetSelectionRange))&&(i.is(t.originSelectionRange)||Q.undefined(t.originSelectionRange))}}(a||(a={})),function(e){e.create=function(e,t,n,r){return{red:e,green:t,blue:n,alpha:r}},e.is=function(e){var t=e;return Q.number(t.red)&&Q.number(t.green)&&Q.number(t.blue)&&Q.number(t.alpha)}}(s||(s={})),function(e){e.create=function(e,t){return{range:e,color:t}},e.is=function(e){var t=e;return i.is(t.range)&&s.is(t.color)}}(u||(u={})),function(e){e.create=function(e,t,n){return{label:e,textEdit:t,additionalTextEdits:n}},e.is=function(e){var t=e;return Q.string(t.label)&&(Q.undefined(t.textEdit)||m.is(t))&&(Q.undefined(t.additionalTextEdits)||Q.typedArray(t.additionalTextEdits,m.is))}}(c||(c={})),function(e){e.Comment="comment",e.Imports="imports",e.Region="region"}(d||(d={})),function(e){e.create=function(e,t,n,r,i){var o={startLine:e,endLine:t};return Q.defined(n)&&(o.startCharacter=n),Q.defined(r)&&(o.endCharacter=r),Q.defined(i)&&(o.kind=i),o},e.is=function(e){var t=e;return Q.number(t.startLine)&&Q.number(t.startLine)&&(Q.undefined(t.startCharacter)||Q.number(t.startCharacter))&&(Q.undefined(t.endCharacter)||Q.number(t.endCharacter))&&(Q.undefined(t.kind)||Q.string(t.kind))}}(f||(f={})),function(e){e.create=function(e,t){return{location:e,message:t}},e.is=function(e){var t=e;return Q.defined(t)&&o.is(t.location)&&Q.string(t.message)}}(l||(l={})),function(e){e.Error=1,e.Warning=2,e.Information=3,e.Hint=4}(g||(g={})),function(e){e.create=function(e,t,n,r,i,o){var a={range:e,message:t};return Q.defined(n)&&(a.severity=n),Q.defined(r)&&(a.code=r),Q.defined(i)&&(a.source=i),Q.defined(o)&&(a.relatedInformation=o),a},e.is=function(e){var t=e;return Q.defined(t)&&i.is(t.range)&&Q.string(t.message)&&(Q.number(t.severity)||Q.undefined(t.severity))&&(Q.number(t.code)||Q.string(t.code)||Q.undefined(t.code))&&(Q.string(t.source)||Q.undefined(t.source))&&(Q.undefined(t.relatedInformation)||Q.typedArray(t.relatedInformation,l.is))}}(h||(h={})),function(e){e.create=function(e,t){for(var n=[],r=2;r<arguments.length;r++)n[r-2]=arguments[r];var i={title:e,command:t};return Q.defined(n)&&n.length>0&&(i.arguments=n),i},e.is=function(e){var t=e;return Q.defined(t)&&Q.string(t.title)&&Q.string(t.command)}}(p||(p={})),function(e){e.replace=function(e,t){return{range:e,newText:t}},e.insert=function(e,t){return{range:{start:e,end:e},newText:t}},e.del=function(e){return{range:e,newText:""}},e.is=function(e){var t=e;return Q.objectLiteral(t)&&Q.string(t.newText)&&i.is(t.range)}}(m||(m={})),function(e){e.create=function(e,t){return{textDocument:e,edits:t}},e.is=function(e){var t=e;return Q.defined(t)&&E.is(t.textDocument)&&Array.isArray(t.edits)}}(v||(v={})),function(e){e.create=function(e,t){var n={kind:"create",uri:e};return void 0===t||void 0===t.overwrite&&void 0===t.ignoreIfExists||(n.options=t),n},e.is=function(e){var t=e;return t&&"create"===t.kind&&Q.string(t.uri)&&(void 0===t.options||(void 0===t.options.overwrite||Q.boolean(t.options.overwrite))&&(void 0===t.options.ignoreIfExists||Q.boolean(t.options.ignoreIfExists)))}}(b||(b={})),function(e){e.create=function(e,t,n){var r={kind:"rename",oldUri:e,newUri:t};return void 0===n||void 0===n.overwrite&&void 0===n.ignoreIfExists||(r.options=n),r},e.is=function(e){var t=e;return t&&"rename"===t.kind&&Q.string(t.oldUri)&&Q.string(t.newUri)&&(void 0===t.options||(void 0===t.options.overwrite||Q.boolean(t.options.overwrite))&&(void 0===t.options.ignoreIfExists||Q.boolean(t.options.ignoreIfExists)))}}(k||(k={})),function(e){e.create=function(e,t){var n={kind:"delete",uri:e};return void 0===t||void 0===t.recursive&&void 0===t.ignoreIfNotExists||(n.options=t),n},e.is=function(e){var t=e;return t&&"delete"===t.kind&&Q.string(t.uri)&&(void 0===t.options||(void 0===t.options.recursive||Q.boolean(t.options.recursive))&&(void 0===t.options.ignoreIfNotExists||Q.boolean(t.options.ignoreIfNotExists)))}}(C||(C={})),function(e){e.is=function(e){var t=e;return t&&(void 0!==t.changes||void 0!==t.documentChanges)&&(void 0===t.documentChanges||t.documentChanges.every((function(e){return Q.string(e.kind)?b.is(e)||k.is(e)||C.is(e):v.is(e)})))}}(_||(_={}));var y,E,x,S,I,A,T,M,P,R,F,j,D,L,O,W,N,U=function(){function e(e){this.edits=e}return e.prototype.insert=function(e,t){this.edits.push(m.insert(e,t))},e.prototype.replace=function(e,t){this.edits.push(m.replace(e,t))},e.prototype.delete=function(e){this.edits.push(m.del(e))},e.prototype.add=function(e){this.edits.push(e)},e.prototype.all=function(){return this.edits},e.prototype.clear=function(){this.edits.splice(0,this.edits.length)},e}();!function(){function e(e){var t=this;this._textEditChanges=Object.create(null),e&&(this._workspaceEdit=e,e.documentChanges?e.documentChanges.forEach((function(e){if(v.is(e)){var n=new U(e.edits);t._textEditChanges[e.textDocument.uri]=n}})):e.changes&&Object.keys(e.changes).forEach((function(n){var r=new U(e.changes[n]);t._textEditChanges[n]=r})))}Object.defineProperty(e.prototype,"edit",{get:function(){return this._workspaceEdit},enumerable:!0,configurable:!0}),e.prototype.getTextEditChange=function(e){if(E.is(e)){if(this._workspaceEdit||(this._workspaceEdit={documentChanges:[]}),!this._workspaceEdit.documentChanges)throw new Error("Workspace edit is not configured for document changes.");var t=e;if(!(r=this._textEditChanges[t.uri])){var n={textDocument:t,edits:i=[]};this._workspaceEdit.documentChanges.push(n),r=new U(i),this._textEditChanges[t.uri]=r}return r}if(this._workspaceEdit||(this._workspaceEdit={changes:Object.create(null)}),!this._workspaceEdit.changes)throw new Error("Workspace edit is not configured for normal text edit changes.");var r;if(!(r=this._textEditChanges[e])){var i=[];this._workspaceEdit.changes[e]=i,r=new U(i),this._textEditChanges[e]=r}return r},e.prototype.createFile=function(e,t){this.checkDocumentChanges(),this._workspaceEdit.documentChanges.push(b.create(e,t))},e.prototype.renameFile=function(e,t,n){this.checkDocumentChanges(),this._workspaceEdit.documentChanges.push(k.create(e,t,n))},e.prototype.deleteFile=function(e,t){this.checkDocumentChanges(),this._workspaceEdit.documentChanges.push(C.create(e,t))},e.prototype.checkDocumentChanges=function(){if(!this._workspaceEdit||!this._workspaceEdit.documentChanges)throw new Error("Workspace edit is not configured for document changes.")}}();!function(e){e.create=function(e){return{uri:e}},e.is=function(e){var t=e;return Q.defined(t)&&Q.string(t.uri)}}(y||(y={})),function(e){e.create=function(e,t){return{uri:e,version:t}},e.is=function(e){var t=e;return Q.defined(t)&&Q.string(t.uri)&&(null===t.version||Q.number(t.version))}}(E||(E={})),function(e){e.create=function(e,t,n,r){return{uri:e,languageId:t,version:n,text:r}},e.is=function(e){var t=e;return Q.defined(t)&&Q.string(t.uri)&&Q.string(t.languageId)&&Q.number(t.version)&&Q.string(t.text)}}(x||(x={})),function(e){e.PlainText="plaintext",e.Markdown="markdown"}(S||(S={})),function(e){e.is=function(t){var n=t;return n===e.PlainText||n===e.Markdown}}(S||(S={})),function(e){e.is=function(e){var t=e;return Q.objectLiteral(e)&&S.is(t.kind)&&Q.string(t.value)}}(I||(I={})),function(e){e.Text=1,e.Method=2,e.Function=3,e.Constructor=4,e.Field=5,e.Variable=6,e.Class=7,e.Interface=8,e.Module=9,e.Property=10,e.Unit=11,e.Value=12,e.Enum=13,e.Keyword=14,e.Snippet=15,e.Color=16,e.File=17,e.Reference=18,e.Folder=19,e.EnumMember=20,e.Constant=21,e.Struct=22,e.Event=23,e.Operator=24,e.TypeParameter=25}(A||(A={})),function(e){e.PlainText=1,e.Snippet=2}(T||(T={})),function(e){e.create=function(e){return{label:e}}}(M||(M={})),function(e){e.create=function(e,t){return{items:e||[],isIncomplete:!!t}}}(P||(P={})),function(e){e.fromPlainText=function(e){return e.replace(/[\\`*_{}[\]()#+\-.!]/g,"\\$&")},e.is=function(e){var t=e;return Q.string(t)||Q.objectLiteral(t)&&Q.string(t.language)&&Q.string(t.value)}}(R||(R={})),function(e){e.is=function(e){var t=e;return!!t&&Q.objectLiteral(t)&&(I.is(t.contents)||R.is(t.contents)||Q.typedArray(t.contents,R.is))&&(void 0===e.range||i.is(e.range))}}(F||(F={})),function(e){e.create=function(e,t){return t?{label:e,documentation:t}:{label:e}}}(j||(j={})),function(e){e.create=function(e,t){for(var n=[],r=2;r<arguments.length;r++)n[r-2]=arguments[r];var i={label:e};return Q.defined(t)&&(i.documentation=t),Q.defined(n)?i.parameters=n:i.parameters=[],i}}(D||(D={})),function(e){e.Text=1,e.Read=2,e.Write=3}(L||(L={})),function(e){e.create=function(e,t){var n={range:e};return Q.number(t)&&(n.kind=t),n}}(O||(O={})),function(e){e.File=1,e.Module=2,e.Namespace=3,e.Package=4,e.Class=5,e.Method=6,e.Property=7,e.Field=8,e.Constructor=9,e.Enum=10,e.Interface=11,e.Function=12,e.Variable=13,e.Constant=14,e.String=15,e.Number=16,e.Boolean=17,e.Array=18,e.Object=19,e.Key=20,e.Null=21,e.EnumMember=22,e.Struct=23,e.Event=24,e.Operator=25,e.TypeParameter=26}(W||(W={})),function(e){e.create=function(e,t,n,r,i){var o={name:e,kind:t,location:{uri:r,range:n}};return i&&(o.containerName=i),o}}(N||(N={}));var V,K,z,H,q,B=function(){};!function(e){e.create=function(e,t,n,r,i,o){var a={name:e,detail:t,kind:n,range:r,selectionRange:i};return void 0!==o&&(a.children=o),a},e.is=function(e){var t=e;return t&&Q.string(t.name)&&Q.number(t.kind)&&i.is(t.range)&&i.is(t.selectionRange)&&(void 0===t.detail||Q.string(t.detail))&&(void 0===t.deprecated||Q.boolean(t.deprecated))&&(void 0===t.children||Array.isArray(t.children))}}(B||(B={})),function(e){e.QuickFix="quickfix",e.Refactor="refactor",e.RefactorExtract="refactor.extract",e.RefactorInline="refactor.inline",e.RefactorRewrite="refactor.rewrite",e.Source="source",e.SourceOrganizeImports="source.organizeImports"}(V||(V={})),function(e){e.create=function(e,t){var n={diagnostics:e};return null!=t&&(n.only=t),n},e.is=function(e){var t=e;return Q.defined(t)&&Q.typedArray(t.diagnostics,h.is)&&(void 0===t.only||Q.typedArray(t.only,Q.string))}}(K||(K={})),function(e){e.create=function(e,t,n){var r={title:e};return p.is(t)?r.command=t:r.edit=t,void 0!==n&&(r.kind=n),r},e.is=function(e){var t=e;return t&&Q.string(t.title)&&(void 0===t.diagnostics||Q.typedArray(t.diagnostics,h.is))&&(void 0===t.kind||Q.string(t.kind))&&(void 0!==t.edit||void 0!==t.command)&&(void 0===t.command||p.is(t.command))&&(void 0===t.edit||_.is(t.edit))}}(z||(z={})),function(e){e.create=function(e,t){var n={range:e};return Q.defined(t)&&(n.data=t),n},e.is=function(e){var t=e;return Q.defined(t)&&i.is(t.range)&&(Q.undefined(t.command)||p.is(t.command))}}(H||(H={})),function(e){e.create=function(e,t){return{tabSize:e,insertSpaces:t}},e.is=function(e){var t=e;return Q.defined(t)&&Q.number(t.tabSize)&&Q.boolean(t.insertSpaces)}}(q||(q={}));var J=function(){};!function(e){e.create=function(e,t,n){return{range:e,target:t,data:n}},e.is=function(e){var t=e;return Q.defined(t)&&i.is(t.range)&&(Q.undefined(t.target)||Q.string(t.target))}}(J||(J={}));var $,G;!function(e){e.create=function(e,t,n,r){return new X(e,t,n,r)},e.is=function(e){var t=e;return!!(Q.defined(t)&&Q.string(t.uri)&&(Q.undefined(t.languageId)||Q.string(t.languageId))&&Q.number(t.lineCount)&&Q.func(t.getText)&&Q.func(t.positionAt)&&Q.func(t.offsetAt))},e.applyEdits=function(e,t){for(var n=e.getText(),r=function e(t,n){if(t.length<=1)return t;var r=t.length/2|0,i=t.slice(0,r),o=t.slice(r);e(i,n),e(o,n);var a=0,s=0,u=0;for(;a<i.length&&s<o.length;){var c=n(i[a],o[s]);t[u++]=c<=0?i[a++]:o[s++]}for(;a<i.length;)t[u++]=i[a++];for(;s<o.length;)t[u++]=o[s++];return t}(t,(function(e,t){var n=e.range.start.line-t.range.start.line;return 0===n?e.range.start.character-t.range.start.character:n})),i=n.length,o=r.length-1;o>=0;o--){var a=r[o],s=e.offsetAt(a.range.start),u=e.offsetAt(a.range.end);if(!(u<=i))throw new Error("Overlapping edit");n=n.substring(0,s)+a.newText+n.substring(u,n.length),i=s}return n}}($||($={})),function(e){e.Manual=1,e.AfterDelay=2,e.FocusOut=3}(G||(G={}));var Q,X=function(){function e(e,t,n,r){this._uri=e,this._languageId=t,this._version=n,this._content=r,this._lineOffsets=null}return Object.defineProperty(e.prototype,"uri",{get:function(){return this._uri},enumerable:!0,configurable:!0}),Object.defineProperty(e.prototype,"languageId",{get:function(){return this._languageId},enumerable:!0,configurable:!0}),Object.defineProperty(e.prototype,"version",{get:function(){return this._version},enumerable:!0,configurable:!0}),e.prototype.getText=function(e){if(e){var t=this.offsetAt(e.start),n=this.offsetAt(e.end);return this._content.substring(t,n)}return this._content},e.prototype.update=function(e,t){this._content=e.text,this._version=t,this._lineOffsets=null},e.prototype.getLineOffsets=function(){if(null===this._lineOffsets){for(var e=[],t=this._content,n=!0,r=0;r<t.length;r++){n&&(e.push(r),n=!1);var i=t.charAt(r);n="\r"===i||"\n"===i,"\r"===i&&r+1<t.length&&"\n"===t.charAt(r+1)&&r++}n&&t.length>0&&e.push(t.length),this._lineOffsets=e}return this._lineOffsets},e.prototype.positionAt=function(e){e=Math.max(Math.min(e,this._content.length),0);var t=this.getLineOffsets(),n=0,i=t.length;if(0===i)return r.create(0,e);for(;n<i;){var o=Math.floor((n+i)/2);t[o]>e?i=o:n=o+1}var a=n-1;return r.create(a,e-t[a])},e.prototype.offsetAt=function(e){var t=this.getLineOffsets();if(e.line>=t.length)return this._content.length;if(e.line<0)return 0;var n=t[e.line],r=e.line+1<t.length?t[e.line+1]:this._content.length;return Math.max(Math.min(n+e.character,r),n)},Object.defineProperty(e.prototype,"lineCount",{get:function(){return this.getLineOffsets().length},enumerable:!0,configurable:!0}),e}();!function(e){var t=Object.prototype.toString;e.defined=function(e){return void 0!==e},e.undefined=function(e){return void 0===e},e.boolean=function(e){return!0===e||!1===e},e.string=function(e){return"[object String]"===t.call(e)},e.number=function(e){return"[object Number]"===t.call(e)},e.func=function(e){return"[object Function]"===t.call(e)},e.objectLiteral=function(e){return null!==e&&"object"==typeof e},e.typedArray=function(e,t){return Array.isArray(e)&&e.every(t)}}(Q||(Q={}));monaco.Uri;var Y=monaco.Range,Z=function(){function e(e,t,n){var r=this;this._languageId=e,this._worker=t,this._disposables=[],this._listener=Object.create(null);var i=function(e){var t,n=e.getModeId();n===r._languageId&&(r._listener[e.uri.toString()]=e.onDidChangeContent((function(){clearTimeout(t),t=setTimeout((function(){return r._doValidate(e.uri,n)}),500)})),r._doValidate(e.uri,n))},o=function(e){monaco.editor.setModelMarkers(e,r._languageId,[]);var t=e.uri.toString(),n=r._listener[t];n&&(n.dispose(),delete r._listener[t])};this._disposables.push(monaco.editor.onDidCreateModel(i)),this._disposables.push(monaco.editor.onWillDisposeModel((function(e){o(e),r._resetSchema(e.uri)}))),this._disposables.push(monaco.editor.onDidChangeModelLanguage((function(e){o(e.model),i(e.model),r._resetSchema(e.model.uri)}))),this._disposables.push(n.onDidChange((function(e){monaco.editor.getModels().forEach((function(e){e.getModeId()===r._languageId&&(o(e),i(e))}))}))),this._disposables.push({dispose:function(){for(var e in monaco.editor.getModels().forEach(o),r._listener)r._listener[e].dispose()}}),monaco.editor.getModels().forEach(i)}return e.prototype.dispose=function(){this._disposables.forEach((function(e){return e&&e.dispose()})),this._disposables=[]},e.prototype._resetSchema=function(e){this._worker().then((function(t){t.resetSchema(e.toString())}))},e.prototype._doValidate=function(e,t){this._worker(e).then((function(n){return n.doValidation(e.toString()).then((function(n){var r=n.map((function(e){return n="number"==typeof(t=e).code?String(t.code):t.code,{severity:ee(t.severity),startLineNumber:t.range.start.line+1,startColumn:t.range.start.character+1,endLineNumber:t.range.end.line+1,endColumn:t.range.end.character+1,message:t.message,code:n,source:t.source};var t,n})),i=monaco.editor.getModel(e);i&&i.getModeId()===t&&monaco.editor.setModelMarkers(i,t,r)}))})).then(void 0,(function(e){console.error(e)}))},e}();function ee(e){switch(e){case g.Error:return monaco.MarkerSeverity.Error;case g.Warning:return monaco.MarkerSeverity.Warning;case g.Information:return monaco.MarkerSeverity.Info;case g.Hint:return monaco.MarkerSeverity.Hint;default:return monaco.MarkerSeverity.Info}}function te(e){if(e)return{character:e.column-1,line:e.lineNumber-1}}function ne(e){if(e)return{start:{line:e.startLineNumber-1,character:e.startColumn-1},end:{line:e.endLineNumber-1,character:e.endColumn-1}}}function re(e){if(e)return new Y(e.start.line+1,e.start.character+1,e.end.line+1,e.end.character+1)}function ie(e){var t=monaco.languages.CompletionItemKind;switch(e){case A.Text:return t.Text;case A.Method:return t.Method;case A.Function:return t.Function;case A.Constructor:return t.Constructor;case A.Field:return t.Field;case A.Variable:return t.Variable;case A.Class:return t.Class;case A.Interface:return t.Interface;case A.Module:return t.Module;case A.Property:return t.Property;case A.Unit:return t.Unit;case A.Value:return t.Value;case A.Enum:return t.Enum;case A.Keyword:return t.Keyword;case A.Snippet:return t.Snippet;case A.Color:return t.Color;case A.File:return t.File;case A.Reference:return t.Reference}return t.Property}function oe(e){if(e)return{range:re(e.range),text:e.newText}}var ae=function(){function e(e){this._worker=e}return Object.defineProperty(e.prototype,"triggerCharacters",{get:function(){return[" ",":"]},enumerable:!0,configurable:!0}),e.prototype.provideCompletionItems=function(e,t,n,r){var i=e.uri;return this._worker(i).then((function(e){return e.doComplete(i.toString(),te(t))})).then((function(n){if(n){var r=e.getWordUntilPosition(t),i=new Y(t.lineNumber,r.startColumn,t.lineNumber,r.endColumn),o=n.items.map((function(e){var t={label:e.label,insertText:e.insertText||e.label,sortText:e.sortText,filterText:e.filterText,documentation:e.documentation,detail:e.detail,range:i,kind:ie(e.kind)};return e.textEdit&&(t.range=re(e.textEdit.range),t.insertText=e.textEdit.newText),e.additionalTextEdits&&(t.additionalTextEdits=e.additionalTextEdits.map(oe)),e.insertTextFormat===T.Snippet&&(t.insertTextRules=monaco.languages.CompletionItemInsertTextRule.InsertAsSnippet),t}));return{isIncomplete:n.isIncomplete,suggestions:o}}}))},e}();function se(e){return"string"==typeof e?{value:e}:(t=e)&&"object"==typeof t&&"string"==typeof t.kind?"plaintext"===e.kind?{value:e.value.replace(/[\\`*_{}[\]()#+\-.!]/g,"\\$&")}:{value:e.value}:{value:"```"+e.language+"\n"+e.value+"\n```\n"};var t}function ue(e){if(e)return Array.isArray(e)?e.map(se):[se(e)]}var ce=function(){function e(e){this._worker=e}return e.prototype.provideHover=function(e,t,n){var r=e.uri;return this._worker(r).then((function(e){return e.doHover(r.toString(),te(t))})).then((function(e){if(e)return{range:re(e.range),contents:ue(e.contents)}}))},e}();function de(e){var t=monaco.languages.SymbolKind;switch(e){case W.File:return t.Array;case W.Module:return t.Module;case W.Namespace:return t.Namespace;case W.Package:return t.Package;case W.Class:return t.Class;case W.Method:return t.Method;case W.Property:return t.Property;case W.Field:return t.Field;case W.Constructor:return t.Constructor;case W.Enum:return t.Enum;case W.Interface:return t.Interface;case W.Function:return t.Function;case W.Variable:return t.Variable;case W.Constant:return t.Constant;case W.String:return t.String;case W.Number:return t.Number;case W.Boolean:return t.Boolean;case W.Array:return t.Array}return t.Function}var fe=function(){function e(e){this._worker=e}return e.prototype.provideDocumentSymbols=function(e,t){var n=e.uri;return this._worker(n).then((function(e){return e.findDocumentSymbols(n.toString())})).then((function(e){if(e)return e.map((function(e){return{name:e.name,detail:"",containerName:e.containerName,kind:de(e.kind),range:re(e.location.range),selectionRange:re(e.location.range),tags:[]}}))}))},e}();function le(e){return{tabSize:e.tabSize,insertSpaces:e.insertSpaces}}var ge,he=function(){function e(e){this._worker=e}return e.prototype.provideDocumentFormattingEdits=function(e,t,n){var r=e.uri;return this._worker(r).then((function(e){return e.format(r.toString(),null,le(t)).then((function(e){if(e&&0!==e.length)return e.map(oe)}))}))},e}(),pe=function(){function e(e){this._worker=e}return e.prototype.provideDocumentRangeFormattingEdits=function(e,t,n,r){var i=e.uri;return this._worker(i).then((function(e){return e.format(i.toString(),ne(t),le(n)).then((function(e){if(e&&0!==e.length)return e.map(oe)}))}))},e}(),me=function(){function e(e){this._worker=e}return e.prototype.provideDocumentColors=function(e,t){var n=e.uri;return this._worker(n).then((function(e){return e.findDocumentColors(n.toString())})).then((function(e){if(e)return e.map((function(e){return{color:e.color,range:re(e.range)}}))}))},e.prototype.provideColorPresentations=function(e,t,n){var r=e.uri;return this._worker(r).then((function(e){return e.getColorPresentations(r.toString(),t.color,ne(t.range))})).then((function(e){if(e)return e.map((function(e){var t={label:e.label};return e.textEdit&&(t.textEdit=oe(e.textEdit)),e.additionalTextEdits&&(t.additionalTextEdits=e.additionalTextEdits.map(oe)),t}))}))},e}(),ve=function(){function e(e){this._worker=e}return e.prototype.provideFoldingRanges=function(e,t,n){var r=e.uri;return this._worker(r).then((function(e){return e.provideFoldingRanges(r.toString(),t)})).then((function(e){if(e)return e.map((function(e){var t={start:e.startLine+1,end:e.endLine+1};return void 0!==e.kind&&(t.kind=function(e){switch(e){case d.Comment:return monaco.languages.FoldingRangeKind.Comment;case d.Imports:return monaco.languages.FoldingRangeKind.Imports;case d.Region:return monaco.languages.FoldingRangeKind.Region}return}(e.kind)),t}))}))},e}();function be(e,t){void 0===t&&(t=!1);var n=0,r=e.length,i="",o=0,a=16,s=0,u=0,c=0,d=0,f=0;function l(t,r){for(var i=0,o=0;i<t||!r;){var a=e.charCodeAt(n);if(a>=48&&a<=57)o=16*o+a-48;else if(a>=65&&a<=70)o=16*o+a-65+10;else{if(!(a>=97&&a<=102))break;o=16*o+a-97+10}n++,i++}return i<t&&(o=-1),o}function g(){if(i="",f=0,o=n,u=s,d=c,n>=r)return o=r,a=17;var t=e.charCodeAt(n);if(ke(t)){do{n++,i+=String.fromCharCode(t),t=e.charCodeAt(n)}while(ke(t));return a=15}if(Ce(t))return n++,i+=String.fromCharCode(t),13===t&&10===e.charCodeAt(n)&&(n++,i+="\n"),s++,c=n,a=14;switch(t){case 123:return n++,a=1;case 125:return n++,a=2;case 91:return n++,a=3;case 93:return n++,a=4;case 58:return n++,a=6;case 44:return n++,a=5;case 34:return n++,i=function(){for(var t="",i=n;;){if(n>=r){t+=e.substring(i,n),f=2;break}var o=e.charCodeAt(n);if(34===o){t+=e.substring(i,n),n++;break}if(92!==o){if(o>=0&&o<=31){if(Ce(o)){t+=e.substring(i,n),f=2;break}f=6}n++}else{if(t+=e.substring(i,n),++n>=r){f=2;break}switch(o=e.charCodeAt(n++)){case 34:t+='"';break;case 92:t+="\\";break;case 47:t+="/";break;case 98:t+="\b";break;case 102:t+="\f";break;case 110:t+="\n";break;case 114:t+="\r";break;case 116:t+="\t";break;case 117:var a=l(4,!0);a>=0?t+=String.fromCharCode(a):f=4;break;default:f=5}i=n}}return t}(),a=10;case 47:var g=n-1;if(47===e.charCodeAt(n+1)){for(n+=2;n<r&&!Ce(e.charCodeAt(n));)n++;return i=e.substring(g,n),a=12}if(42===e.charCodeAt(n+1)){n+=2;for(var p=r-1,m=!1;n<p;){var v=e.charCodeAt(n);if(42===v&&47===e.charCodeAt(n+1)){n+=2,m=!0;break}n++,Ce(v)&&(13===v&&10===e.charCodeAt(n)&&n++,s++,c=n)}return m||(n++,f=1),i=e.substring(g,n),a=13}return i+=String.fromCharCode(t),n++,a=16;case 45:if(i+=String.fromCharCode(t),++n===r||!_e(e.charCodeAt(n)))return a=16;case 48:case 49:case 50:case 51:case 52:case 53:case 54:case 55:case 56:case 57:return i+=function(){var t=n;if(48===e.charCodeAt(n))n++;else for(n++;n<e.length&&_e(e.charCodeAt(n));)n++;if(n<e.length&&46===e.charCodeAt(n)){if(!(++n<e.length&&_e(e.charCodeAt(n))))return f=3,e.substring(t,n);for(n++;n<e.length&&_e(e.charCodeAt(n));)n++}var r=n;if(n<e.length&&(69===e.charCodeAt(n)||101===e.charCodeAt(n)))if((++n<e.length&&43===e.charCodeAt(n)||45===e.charCodeAt(n))&&n++,n<e.length&&_e(e.charCodeAt(n))){for(n++;n<e.length&&_e(e.charCodeAt(n));)n++;r=n}else f=3;return e.substring(t,r)}(),a=11;default:for(;n<r&&h(t);)n++,t=e.charCodeAt(n);if(o!==n){switch(i=e.substring(o,n)){case"true":return a=8;case"false":return a=9;case"null":return a=7}return a=16}return i+=String.fromCharCode(t),n++,a=16}}function h(e){if(ke(e)||Ce(e))return!1;switch(e){case 125:case 93:case 123:case 91:case 34:case 58:case 44:case 47:return!1}return!0}return{setPosition:function(e){n=e,i="",o=0,a=16,f=0},getPosition:function(){return n},scan:t?function(){var e;do{e=g()}while(e>=12&&e<=15);return e}:g,getToken:function(){return a},getTokenValue:function(){return i},getTokenOffset:function(){return o},getTokenLength:function(){return n-o},getTokenStartLine:function(){return u},getTokenStartCharacter:function(){return o-d},getTokenError:function(){return f}}}function ke(e){return 32===e||9===e||11===e||12===e||160===e||5760===e||e>=8192&&e<=8203||8239===e||8287===e||12288===e||65279===e}function Ce(e){return 10===e||13===e||8232===e||8233===e}function _e(e){return e>=48&&e<=57}!function(e){e.DEFAULT={allowTrailingComma:!1}}(ge||(ge={}));var we=be;function ye(e){return{getInitialState:function(){return new Ee(null,null,!1)},tokenize:function(t,n,r,i){return function(e,t,n,r,i){void 0===r&&(r=0);var o=0,a=!1;switch(n.scanError){case 2:t='"'+t,o=1;break;case 1:t="/*"+t,o=2}var s,u,c=we(t),d=n.lastWasColon;u={tokens:[],endState:n.clone()};for(;;){var f=r+c.getPosition(),l="";if(17===(s=c.scan()))break;if(f===r+c.getPosition())throw new Error("Scanner did not advance, next 3 characters are: "+t.substr(c.getPosition(),3));switch(a&&(f-=o),a=o>0,s){case 1:case 2:l="delimiter.bracket.json",d=!1;break;case 3:case 4:l="delimiter.array.json",d=!1;break;case 6:l="delimiter.colon.json",d=!0;break;case 5:l="delimiter.comma.json",d=!1;break;case 8:case 9:case 7:l="keyword.json",d=!1;break;case 10:l=d?"string.value.json":"string.key.json",d=!1;break;case 11:l="number.json",d=!1}if(e)switch(s){case 12:l="comment.line.json";break;case 13:l="comment.block.json"}u.endState=new Ee(n.getStateData(),c.getTokenError(),d),u.tokens.push({startIndex:f,scopes:l})}return u}(e,t,n,r)}}}var Ee=function(){function e(e,t,n){this._state=e,this.scanError=t,this.lastWasColon=n}return e.prototype.clone=function(){return new e(this._state,this.scanError,this.lastWasColon)},e.prototype.equals=function(t){return t===this||!!(t&&t instanceof e)&&(this.scanError===t.scanError&&this.lastWasColon===t.lastWasColon)},e.prototype.getStateData=function(){return this._state},e.prototype.setStateData=function(e){this._state=e},e}();function xe(e){var t=[],n=[],r=new w(e);t.push(r);var i=function(){for(var e=[],t=0;t<arguments.length;t++)e[t]=arguments[t];return r.getLanguageServiceWorker.apply(r,e)};function o(){var t=e.languageId,r=e.modeConfiguration;Ie(n),r.documentFormattingEdits&&n.push(monaco.languages.registerDocumentFormattingEditProvider(t,new he(i))),r.documentRangeFormattingEdits&&n.push(monaco.languages.registerDocumentRangeFormattingEditProvider(t,new pe(i))),r.completionItems&&n.push(monaco.languages.registerCompletionItemProvider(t,new ae(i))),r.hovers&&n.push(monaco.languages.registerHoverProvider(t,new ce(i))),r.documentSymbols&&n.push(monaco.languages.registerDocumentSymbolProvider(t,new fe(i))),r.tokens&&n.push(monaco.languages.setTokensProvider(t,ye(!0))),r.colors&&n.push(monaco.languages.registerColorProvider(t,new me(i))),r.foldingRanges&&n.push(monaco.languages.registerFoldingRangeProvider(t,new ve(i))),r.diagnostics&&n.push(new Z(t,i,e))}o(),t.push(monaco.languages.setLanguageConfiguration(e.languageId,Ae));var a=e.modeConfiguration;return e.onDidChange((function(e){e.modeConfiguration!==a&&(a=e.modeConfiguration,o())})),t.push(Se(n)),Se(t)}function Se(e){return{dispose:function(){return Ie(e)}}}function Ie(e){for(;e.length;)e.pop().dispose()}var Ae={wordPattern:/(-?\d*\.\d\w*)|([^\[\{\]\}\:\"\,\s]+)/g,comments:{lineComment:"//",blockComment:["/*","*/"]},brackets:[["{","}"],["[","]"]],autoClosingPairs:[{open:"{",close:"}",notIn:["string"]},{open:"[",close:"]",notIn:["string"]},{open:'"',close:'"',notIn:["string"]}]}}}]);
//# sourceMappingURL=207-53cb93fcd8760faa2564.js.map