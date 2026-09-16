grammar aura;

// ============================================================================ PROGRAM AND
// TOP-LEVEL ============================================================================

program: (importDecl | declaration | statement)* EOF;

importDecl:
	'import' modulePath ('as' IDENT)?					# ImportModule
	| 'import' modulePath '{' importNames '}'			# ImportNames
	| 'from' modulePath 'import' importItems			# FromImport;

modulePath: IDENT ('.' IDENT)*;

importNames: importName (',' importName)* ','?;

importName: IDENT ('as' IDENT)?;

importItems: importName (',' importName)* ','? | '*';

// ============================================================================ DECLARATIONS
// ============================================================================

declaration:
	varDecl
	| constDecl
	| funcDecl
	| classDecl
	| traitDecl
	| typeDecl
	| moduleDecl;

varDecl:
	'let' 'mut'? IDENT typeAnnotation? ('=' expression)? ';';
constDecl: 'const' IDENT typeAnnotation? '=' expression ';';

funcDecl:
	decorator* 'async'? 'def' IDENT typeParams? '(' parameterList? ')' returnType? (
		'{' statement* '}'
		| '=' expression
	);

typeParams: '[' IDENT (',' IDENT)* ']';

parameterList: parameter (',' parameter)*;

parameter:
	IDENT typeAnnotation? ('=' expression)?	# PositionalParam
	| '*' IDENT								# VariadicParam
	| '*'									# KeywordOnlyMarker
	| '**' IDENT							# KwargParam;

returnType: '->' typeAnnotation;

classDecl:
	decorator* 'class' IDENT typeParams? (
		'(' typeAnnotation ')'
	)? ('implements' typeAnnotation (',' typeAnnotation)*)? '{' classBody* '}';

classBody:
	IDENT typeAnnotation? ('=' expression)?	# FieldDef
	| methodDef								# ClassMethodDef
	| propertyDef							# ClassPropertyDef;

methodDef:
	'@' 'staticmethod'? '@' 'classmethod'? 'def' IDENT '(' parameterList? ')' returnType? (
		'{' statement* '}'
		| '=' expression
	);

propertyDef:
	'@' 'property' 'def' IDENT '(' 'self' ')' returnType? (
		'{' statement* '}'
		| '=' expression
	);

traitDecl: 'trait' IDENT typeParams? '{' traitMember* '}';

traitMember:
	'def' IDENT '(' parameterList? ')' returnType? ';'
	| '@' 'property' 'def' IDENT '(' 'self' ')' returnType? ';';

typeDecl: 'type' IDENT typeParams? '=' typeAnnotation ';';

moduleDecl: 'module' IDENT '{' moduleMember* '}';

moduleMember: 'export'? declaration | 'export'? statement;

decorator: '@' IDENT ('(' argumentList? ')')?;

// ============================================================================ STATEMENTS
// ============================================================================

statement:
	varDecl
	| constDecl
	| exprStmt
	| ifStmt
	| unlessStmt
	| guardStmt
	| whileStmt
	| untilStmt
	| forStmt
	| loopStmt
	| breakStmt
	| continueStmt
	| returnStmt
	| tryStmt
	| withStmt
	| matchStmt
	| returnStmt
	| tryStmt
	| withStmt
	| matchStmt
	| blockStmt
	| assertStmt;

assertStmt: 'assert' expression (',' expression)? ';';

exprStmt: expression ';';

ifStmt:
	'if' expression '{' statement* '}' (
		'else' 'if' expression '{' statement* '}'
	)* ('else' '{' statement* '}')?;

unlessStmt:
	'unless' expression '{' statement* '}' (
		'else' '{' statement* '}'
	)?;

guardStmt: 'guard' expression 'else' '{' statement* '}';

whileStmt: 'while' expression '{' statement* '}';
untilStmt: 'until' expression '{' statement* '}';

forStmt:
	'for' pattern 'in' expression ('step' expression)? '{' statement* '}';

loopStmt: 'loop' '{' statement* '}';

breakStmt: 'break' ';';
continueStmt: 'continue' ';';

returnStmt: 'return' expression? ';';

tryStmt:
	'try' '{' statement* '}' (
		'catch' exceptionPattern '{' statement* '}'
	)* ('finally' '{' statement* '}')?;

exceptionPattern: typeAnnotation ('as' IDENT)? |;

withStmt: 'with' withItem (',' withItem)* '{' statement* '}';

withItem: expression ('as' IDENT)?;

matchStmt: 'match' expression '{' matchCase+ '}';

matchCase: 'case' pattern ('if' expression)? '{' statement* '}';

blockStmt: '{' statement* '}';

// ============================================================================ EXPRESSIONS
// ============================================================================

expression: assignment;

assignment: pipe (assignOp pipe)*;

assignOp:
	'='
	| '+='
	| '-='
	| '*='
	| '/='
	| '%='
	| '**='
	| '&='
	| '|='
	| '^='
	| '<<='
	| '>>='
	| '??=';

pipe: ternary ('|>' ternary)*;

ternary: elvis ('?' expression ':' expression)?;

elvis: coalesce ('?:' coalesce)*;

coalesce: logicalOr ('??' logicalOr)*;

logicalOr: logicalAnd ('or' logicalAnd)*;

logicalAnd: bitwiseOr ('and' bitwiseOr)*;

bitwiseOr: bitwiseXor ('|' bitwiseXor)*;

bitwiseXor: bitwiseAnd ('^' bitwiseAnd)*;

bitwiseAnd: equality ('&' equality)*;

equality: comparison (('==' | '!=' | 'is' | 'in') comparison)*;

comparison: rangeExpr (('<' | '>' | '<=' | '>=') rangeExpr)*;

rangeExpr:
	additive '..' additive ('step' additive)?	# InclusiveRange
	| additive '..<' additive					# ExclusiveRange
	| additive '..' ('step' additive)?			# InfiniteRange
	| additive									# RangeBase;

shift: additive (('<<' | '>>') additive)*;

additive: multiplicative (('+' | '-') multiplicative)*;

multiplicative: castExpr (('*' | '/' | '%') castExpr)*;

castExpr: unary ('as' typeAnnotation)*;

unary: ('not' | '-' | '+' | '~') unary | exponentiation;

exponentiation: postfix ('**' postfix)*;

postfix: primary ( call | index | member | safeNav | spreadOp)*;

call: '(' argumentList? ')';
index: '[' expression ']';
member: '.' IDENT;
safeNav: '?.' IDENT | '?[' expression ']';
spreadOp: ('*' | '**' | '...') IDENT;

primary:
	literal
	| IDENT
	| '(' expression ')'
	| listLiteral
	| dictLiteral
	| setLiteral
	| tupleLiteral
	| lambdaExpr
	| comprehension
	| matchExpr
	| blockExpr;

literal: NUMBER | STRING | 'true' | 'false' | 'null' | 'none';

argumentList: argument (',' argument)*;

argument:
	expression						# PositionalArg
	| IDENT ('=' | ':') expression	# KeywordArg;

listLiteral:
	'[' ']'											# EmptyList
	| '[' expression (',' expression)* ']'			# NonEmptyList
	| '[' expression 'for' comprehensionClause+ ']'	# ListComprehension;

dictLiteral:
	'{' '}'										# EmptyDict
	| '{' kvPair (',' kvPair)* '}'				# NonEmptyDict
	| '{' kvPair 'for' comprehensionClause+ '}'	# DictComp;

kvPair: expression ':' expression;

setLiteral:
	'{' expression (',' expression)+ '}'
	| '{' expression 'for' comprehensionClause+ '}';

tupleLiteral:
	'(' ')'									# EmptyTuple
	| '(' expression (',' expression)* ')'	# NonEmptyTuple;

lambdaExpr:
	'(' parameterList? ')' '=>' (expression | '{' statement* '}')
	| IDENT '=>' (expression | '{' statement* '}');

comprehension:
	'[' expression 'for' comprehensionClause+ ']'	# ListComp
	| '{' expression 'for' comprehensionClause+ '}'	# SetComp;

comprehensionClause:
	'for' pattern 'in' expression ('if' expression)*;

matchExpr: 'match' expression '{' matchCase+ '}';

blockExpr: '{' statement* '}';

// ============================================================================ PATTERNS (for
// matching and destructuring)
// ============================================================================

pattern: orPattern;

orPattern: asPattern ('|' asPattern)*;

asPattern: constructorOrLiteral ('as' IDENT)?;

constructorOrLiteral:
	IDENT '(' patternList? ')'	# ConstructorPattern
	| '[' patternList? ']'		# ListPattern
	| '{' fieldPatterns? '}'	# DictPattern
	| literal					# LiteralPattern
	| IDENT						# IdentifierPattern
	| '_'						# WildcardPattern;

patternList: pattern (',' pattern)* (',' '*' IDENT)?;

fieldPatterns: fieldPattern (',' fieldPattern)*;

fieldPattern: IDENT (':' pattern)?;

// ============================================================================ TYPE ANNOTATIONS
// ============================================================================

typeAnnotation: unionType;

unionType: primaryType ('|' primaryType)*;

primaryType:
	IDENT typeArgs? '?'?												# NamedOrGenericType
	| '(' typeAnnotation (',' typeAnnotation)* ')' '->' typeAnnotation	# FunctionType
	| '[' typeAnnotation ']'											# ListType
	| '[' typeAnnotation ',' typeAnnotation ']'							# DictType
	| '{' fieldTypes? '}'												# StructuralType;

typeArgs: '[' typeAnnotation (',' typeAnnotation)* ']';

fieldTypes: fieldType (',' fieldType)*;

fieldType: IDENT ':' typeAnnotation;

// ============================================================================ LEXICAL RULES
// ============================================================================

IDENT: [a-zA-Z_][a-zA-Z0-9_]*;

NUMBER:
	[0-9]+ '_'? [0-9]* ('.' [0-9]+ ('e' [+-]? [0-9]+)?)?
	| '0x' [0-9a-fA-F]+
	| '0o' [0-7]+
	| '0b' [01]+;

STRING:
	'"' (~["\\\r\n] | '\\' .)* '"'
	| '\'' (~['\\\r\n] | '\\' .)* '\''
	| 'f"' (~["\\\r\n] | '\\' .)* '"'
	| '"""' (~'"' | '"' ~'"' | '""' ~'"')* '"""';

WS: [ \t\r\n]+ -> skip;
COMMENT: '//' ~[\r\n]* -> skip;
BLOCK_COMMENT: '/*' .*? '*/' -> skip;
