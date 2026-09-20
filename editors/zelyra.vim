" Vim/Neovim syntax file for Zelyra (.zyl)
" Language: Zelyra
" Maintainer: Stefan Siedelmann (stefan@siedelmann.com)

if exists("b:current_syntax")
  finish
endif

syn keyword zylKeyword fn page schema let mut const type struct enum impl use mod
syn keyword zylKeyword html sql route form
syn keyword zylConditional if else match
syn keyword zylRepeat for in while break continue return
syn keyword zylType Int Float Text String Bool List Map Option Result Email Date DateTime Void
syn keyword zylBoolean true false null none
syn keyword zylConstant Ok Err Some None

syn match zylDecorator "@[a-zA-Z_][a-zA-Z0-9_]*"
syn match zylFunction "\v<fn>\s+\zs<[a-zA-Z_][a-zA-Z0-9_]*>"
syn match zylOperator "->\|=>\|==\|!=\|<=\|>=\|&&\|||\|=\|+\|-\|\*\|\/\|%"
syn match zylNumber "\v<\d+(\.\d+)?>"

syn match zylComment "//.*$"
syn region zylComment start="/\*" end="\*/"

syn region zylString start='"' skip='\\"' end='"' contains=zylInterpolation
syn region zylInterpolation contained start="{" end="}" contains=ALLBUT,zylComment

hi def link zylKeyword Keyword
hi def link zylConditional Conditional
hi def link zylRepeat Repeat
hi def link zylType Type
hi def link zylBoolean Boolean
hi def link zylConstant Constant
hi def link zylDecorator PreProc
hi def link zylFunction Function
hi def link zylOperator Operator
hi def link zylNumber Number
hi def link zylComment Comment
hi def link zylString String
hi def link zylInterpolation Special

let b:current_syntax = "zelyra"
