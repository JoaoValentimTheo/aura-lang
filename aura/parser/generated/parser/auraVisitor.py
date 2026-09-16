# Generated from parser/aura.g4 by ANTLR 4.13.2
from antlr4 import *
if "." in __name__:
    from .auraParser import auraParser
else:
    from auraParser import auraParser

# This class defines a complete generic visitor for a parse tree produced by auraParser.

class auraVisitor(ParseTreeVisitor):

    # Visit a parse tree produced by auraParser#program.
    def visitProgram(self, ctx:auraParser.ProgramContext):
        return self.visitChildren(ctx)


    # Visit a parse tree produced by auraParser#ImportModule.
    def visitImportModule(self, ctx:auraParser.ImportModuleContext):
        return self.visitChildren(ctx)


    # Visit a parse tree produced by auraParser#FromImport.
    def visitFromImport(self, ctx:auraParser.FromImportContext):
        return self.visitChildren(ctx)


    # Visit a parse tree produced by auraParser#modulePath.
    def visitModulePath(self, ctx:auraParser.ModulePathContext):
        return self.visitChildren(ctx)


    # Visit a parse tree produced by auraParser#importItems.
    def visitImportItems(self, ctx:auraParser.ImportItemsContext):
        return self.visitChildren(ctx)


    # Visit a parse tree produced by auraParser#declaration.
    def visitDeclaration(self, ctx:auraParser.DeclarationContext):
        return self.visitChildren(ctx)


    # Visit a parse tree produced by auraParser#varDecl.
    def visitVarDecl(self, ctx:auraParser.VarDeclContext):
        return self.visitChildren(ctx)


    # Visit a parse tree produced by auraParser#constDecl.
    def visitConstDecl(self, ctx:auraParser.ConstDeclContext):
        return self.visitChildren(ctx)


    # Visit a parse tree produced by auraParser#funcDecl.
    def visitFuncDecl(self, ctx:auraParser.FuncDeclContext):
        return self.visitChildren(ctx)


    # Visit a parse tree produced by auraParser#typeParams.
    def visitTypeParams(self, ctx:auraParser.TypeParamsContext):
        return self.visitChildren(ctx)


    # Visit a parse tree produced by auraParser#parameterList.
    def visitParameterList(self, ctx:auraParser.ParameterListContext):
        return self.visitChildren(ctx)


    # Visit a parse tree produced by auraParser#PositionalParam.
    def visitPositionalParam(self, ctx:auraParser.PositionalParamContext):
        return self.visitChildren(ctx)


    # Visit a parse tree produced by auraParser#VariadicParam.
    def visitVariadicParam(self, ctx:auraParser.VariadicParamContext):
        return self.visitChildren(ctx)


    # Visit a parse tree produced by auraParser#KeywordOnlyMarker.
    def visitKeywordOnlyMarker(self, ctx:auraParser.KeywordOnlyMarkerContext):
        return self.visitChildren(ctx)


    # Visit a parse tree produced by auraParser#KwargParam.
    def visitKwargParam(self, ctx:auraParser.KwargParamContext):
        return self.visitChildren(ctx)


    # Visit a parse tree produced by auraParser#returnType.
    def visitReturnType(self, ctx:auraParser.ReturnTypeContext):
        return self.visitChildren(ctx)


    # Visit a parse tree produced by auraParser#classDecl.
    def visitClassDecl(self, ctx:auraParser.ClassDeclContext):
        return self.visitChildren(ctx)


    # Visit a parse tree produced by auraParser#FieldDef.
    def visitFieldDef(self, ctx:auraParser.FieldDefContext):
        return self.visitChildren(ctx)


    # Visit a parse tree produced by auraParser#ClassMethodDef.
    def visitClassMethodDef(self, ctx:auraParser.ClassMethodDefContext):
        return self.visitChildren(ctx)


    # Visit a parse tree produced by auraParser#ClassPropertyDef.
    def visitClassPropertyDef(self, ctx:auraParser.ClassPropertyDefContext):
        return self.visitChildren(ctx)


    # Visit a parse tree produced by auraParser#methodDef.
    def visitMethodDef(self, ctx:auraParser.MethodDefContext):
        return self.visitChildren(ctx)


    # Visit a parse tree produced by auraParser#propertyDef.
    def visitPropertyDef(self, ctx:auraParser.PropertyDefContext):
        return self.visitChildren(ctx)


    # Visit a parse tree produced by auraParser#traitDecl.
    def visitTraitDecl(self, ctx:auraParser.TraitDeclContext):
        return self.visitChildren(ctx)


    # Visit a parse tree produced by auraParser#traitMember.
    def visitTraitMember(self, ctx:auraParser.TraitMemberContext):
        return self.visitChildren(ctx)


    # Visit a parse tree produced by auraParser#typeDecl.
    def visitTypeDecl(self, ctx:auraParser.TypeDeclContext):
        return self.visitChildren(ctx)


    # Visit a parse tree produced by auraParser#moduleDecl.
    def visitModuleDecl(self, ctx:auraParser.ModuleDeclContext):
        return self.visitChildren(ctx)


    # Visit a parse tree produced by auraParser#moduleMember.
    def visitModuleMember(self, ctx:auraParser.ModuleMemberContext):
        return self.visitChildren(ctx)


    # Visit a parse tree produced by auraParser#decorator.
    def visitDecorator(self, ctx:auraParser.DecoratorContext):
        return self.visitChildren(ctx)


    # Visit a parse tree produced by auraParser#statement.
    def visitStatement(self, ctx:auraParser.StatementContext):
        return self.visitChildren(ctx)


    # Visit a parse tree produced by auraParser#assertStmt.
    def visitAssertStmt(self, ctx:auraParser.AssertStmtContext):
        return self.visitChildren(ctx)


    # Visit a parse tree produced by auraParser#exprStmt.
    def visitExprStmt(self, ctx:auraParser.ExprStmtContext):
        return self.visitChildren(ctx)


    # Visit a parse tree produced by auraParser#ifStmt.
    def visitIfStmt(self, ctx:auraParser.IfStmtContext):
        return self.visitChildren(ctx)


    # Visit a parse tree produced by auraParser#unlessStmt.
    def visitUnlessStmt(self, ctx:auraParser.UnlessStmtContext):
        return self.visitChildren(ctx)


    # Visit a parse tree produced by auraParser#guardStmt.
    def visitGuardStmt(self, ctx:auraParser.GuardStmtContext):
        return self.visitChildren(ctx)


    # Visit a parse tree produced by auraParser#whileStmt.
    def visitWhileStmt(self, ctx:auraParser.WhileStmtContext):
        return self.visitChildren(ctx)


    # Visit a parse tree produced by auraParser#untilStmt.
    def visitUntilStmt(self, ctx:auraParser.UntilStmtContext):
        return self.visitChildren(ctx)


    # Visit a parse tree produced by auraParser#forStmt.
    def visitForStmt(self, ctx:auraParser.ForStmtContext):
        return self.visitChildren(ctx)


    # Visit a parse tree produced by auraParser#loopStmt.
    def visitLoopStmt(self, ctx:auraParser.LoopStmtContext):
        return self.visitChildren(ctx)


    # Visit a parse tree produced by auraParser#breakStmt.
    def visitBreakStmt(self, ctx:auraParser.BreakStmtContext):
        return self.visitChildren(ctx)


    # Visit a parse tree produced by auraParser#continueStmt.
    def visitContinueStmt(self, ctx:auraParser.ContinueStmtContext):
        return self.visitChildren(ctx)


    # Visit a parse tree produced by auraParser#returnStmt.
    def visitReturnStmt(self, ctx:auraParser.ReturnStmtContext):
        return self.visitChildren(ctx)


    # Visit a parse tree produced by auraParser#tryStmt.
    def visitTryStmt(self, ctx:auraParser.TryStmtContext):
        return self.visitChildren(ctx)


    # Visit a parse tree produced by auraParser#exceptionPattern.
    def visitExceptionPattern(self, ctx:auraParser.ExceptionPatternContext):
        return self.visitChildren(ctx)


    # Visit a parse tree produced by auraParser#withStmt.
    def visitWithStmt(self, ctx:auraParser.WithStmtContext):
        return self.visitChildren(ctx)


    # Visit a parse tree produced by auraParser#withItem.
    def visitWithItem(self, ctx:auraParser.WithItemContext):
        return self.visitChildren(ctx)


    # Visit a parse tree produced by auraParser#matchStmt.
    def visitMatchStmt(self, ctx:auraParser.MatchStmtContext):
        return self.visitChildren(ctx)


    # Visit a parse tree produced by auraParser#matchCase.
    def visitMatchCase(self, ctx:auraParser.MatchCaseContext):
        return self.visitChildren(ctx)


    # Visit a parse tree produced by auraParser#blockStmt.
    def visitBlockStmt(self, ctx:auraParser.BlockStmtContext):
        return self.visitChildren(ctx)


    # Visit a parse tree produced by auraParser#expression.
    def visitExpression(self, ctx:auraParser.ExpressionContext):
        return self.visitChildren(ctx)


    # Visit a parse tree produced by auraParser#assignment.
    def visitAssignment(self, ctx:auraParser.AssignmentContext):
        return self.visitChildren(ctx)


    # Visit a parse tree produced by auraParser#assignOp.
    def visitAssignOp(self, ctx:auraParser.AssignOpContext):
        return self.visitChildren(ctx)


    # Visit a parse tree produced by auraParser#pipe.
    def visitPipe(self, ctx:auraParser.PipeContext):
        return self.visitChildren(ctx)


    # Visit a parse tree produced by auraParser#ternary.
    def visitTernary(self, ctx:auraParser.TernaryContext):
        return self.visitChildren(ctx)


    # Visit a parse tree produced by auraParser#elvis.
    def visitElvis(self, ctx:auraParser.ElvisContext):
        return self.visitChildren(ctx)


    # Visit a parse tree produced by auraParser#coalesce.
    def visitCoalesce(self, ctx:auraParser.CoalesceContext):
        return self.visitChildren(ctx)


    # Visit a parse tree produced by auraParser#logicalOr.
    def visitLogicalOr(self, ctx:auraParser.LogicalOrContext):
        return self.visitChildren(ctx)


    # Visit a parse tree produced by auraParser#logicalAnd.
    def visitLogicalAnd(self, ctx:auraParser.LogicalAndContext):
        return self.visitChildren(ctx)


    # Visit a parse tree produced by auraParser#bitwiseOr.
    def visitBitwiseOr(self, ctx:auraParser.BitwiseOrContext):
        return self.visitChildren(ctx)


    # Visit a parse tree produced by auraParser#bitwiseXor.
    def visitBitwiseXor(self, ctx:auraParser.BitwiseXorContext):
        return self.visitChildren(ctx)


    # Visit a parse tree produced by auraParser#bitwiseAnd.
    def visitBitwiseAnd(self, ctx:auraParser.BitwiseAndContext):
        return self.visitChildren(ctx)


    # Visit a parse tree produced by auraParser#equality.
    def visitEquality(self, ctx:auraParser.EqualityContext):
        return self.visitChildren(ctx)


    # Visit a parse tree produced by auraParser#comparison.
    def visitComparison(self, ctx:auraParser.ComparisonContext):
        return self.visitChildren(ctx)


    # Visit a parse tree produced by auraParser#InclusiveRange.
    def visitInclusiveRange(self, ctx:auraParser.InclusiveRangeContext):
        return self.visitChildren(ctx)


    # Visit a parse tree produced by auraParser#ExclusiveRange.
    def visitExclusiveRange(self, ctx:auraParser.ExclusiveRangeContext):
        return self.visitChildren(ctx)


    # Visit a parse tree produced by auraParser#InfiniteRange.
    def visitInfiniteRange(self, ctx:auraParser.InfiniteRangeContext):
        return self.visitChildren(ctx)


    # Visit a parse tree produced by auraParser#RangeBase.
    def visitRangeBase(self, ctx:auraParser.RangeBaseContext):
        return self.visitChildren(ctx)


    # Visit a parse tree produced by auraParser#shift.
    def visitShift(self, ctx:auraParser.ShiftContext):
        return self.visitChildren(ctx)


    # Visit a parse tree produced by auraParser#additive.
    def visitAdditive(self, ctx:auraParser.AdditiveContext):
        return self.visitChildren(ctx)


    # Visit a parse tree produced by auraParser#multiplicative.
    def visitMultiplicative(self, ctx:auraParser.MultiplicativeContext):
        return self.visitChildren(ctx)


    # Visit a parse tree produced by auraParser#castExpr.
    def visitCastExpr(self, ctx:auraParser.CastExprContext):
        return self.visitChildren(ctx)


    # Visit a parse tree produced by auraParser#unary.
    def visitUnary(self, ctx:auraParser.UnaryContext):
        return self.visitChildren(ctx)


    # Visit a parse tree produced by auraParser#exponentiation.
    def visitExponentiation(self, ctx:auraParser.ExponentiationContext):
        return self.visitChildren(ctx)


    # Visit a parse tree produced by auraParser#postfix.
    def visitPostfix(self, ctx:auraParser.PostfixContext):
        return self.visitChildren(ctx)


    # Visit a parse tree produced by auraParser#call.
    def visitCall(self, ctx:auraParser.CallContext):
        return self.visitChildren(ctx)


    # Visit a parse tree produced by auraParser#index.
    def visitIndex(self, ctx:auraParser.IndexContext):
        return self.visitChildren(ctx)


    # Visit a parse tree produced by auraParser#member.
    def visitMember(self, ctx:auraParser.MemberContext):
        return self.visitChildren(ctx)


    # Visit a parse tree produced by auraParser#safeNav.
    def visitSafeNav(self, ctx:auraParser.SafeNavContext):
        return self.visitChildren(ctx)


    # Visit a parse tree produced by auraParser#spreadOp.
    def visitSpreadOp(self, ctx:auraParser.SpreadOpContext):
        return self.visitChildren(ctx)


    # Visit a parse tree produced by auraParser#primary.
    def visitPrimary(self, ctx:auraParser.PrimaryContext):
        return self.visitChildren(ctx)


    # Visit a parse tree produced by auraParser#literal.
    def visitLiteral(self, ctx:auraParser.LiteralContext):
        return self.visitChildren(ctx)


    # Visit a parse tree produced by auraParser#argumentList.
    def visitArgumentList(self, ctx:auraParser.ArgumentListContext):
        return self.visitChildren(ctx)


    # Visit a parse tree produced by auraParser#PositionalArg.
    def visitPositionalArg(self, ctx:auraParser.PositionalArgContext):
        return self.visitChildren(ctx)


    # Visit a parse tree produced by auraParser#KeywordArg.
    def visitKeywordArg(self, ctx:auraParser.KeywordArgContext):
        return self.visitChildren(ctx)


    # Visit a parse tree produced by auraParser#EmptyList.
    def visitEmptyList(self, ctx:auraParser.EmptyListContext):
        return self.visitChildren(ctx)


    # Visit a parse tree produced by auraParser#NonEmptyList.
    def visitNonEmptyList(self, ctx:auraParser.NonEmptyListContext):
        return self.visitChildren(ctx)


    # Visit a parse tree produced by auraParser#ListComprehension.
    def visitListComprehension(self, ctx:auraParser.ListComprehensionContext):
        return self.visitChildren(ctx)


    # Visit a parse tree produced by auraParser#EmptyDict.
    def visitEmptyDict(self, ctx:auraParser.EmptyDictContext):
        return self.visitChildren(ctx)


    # Visit a parse tree produced by auraParser#NonEmptyDict.
    def visitNonEmptyDict(self, ctx:auraParser.NonEmptyDictContext):
        return self.visitChildren(ctx)


    # Visit a parse tree produced by auraParser#DictComp.
    def visitDictComp(self, ctx:auraParser.DictCompContext):
        return self.visitChildren(ctx)


    # Visit a parse tree produced by auraParser#kvPair.
    def visitKvPair(self, ctx:auraParser.KvPairContext):
        return self.visitChildren(ctx)


    # Visit a parse tree produced by auraParser#setLiteral.
    def visitSetLiteral(self, ctx:auraParser.SetLiteralContext):
        return self.visitChildren(ctx)


    # Visit a parse tree produced by auraParser#EmptyTuple.
    def visitEmptyTuple(self, ctx:auraParser.EmptyTupleContext):
        return self.visitChildren(ctx)


    # Visit a parse tree produced by auraParser#NonEmptyTuple.
    def visitNonEmptyTuple(self, ctx:auraParser.NonEmptyTupleContext):
        return self.visitChildren(ctx)


    # Visit a parse tree produced by auraParser#lambdaExpr.
    def visitLambdaExpr(self, ctx:auraParser.LambdaExprContext):
        return self.visitChildren(ctx)


    # Visit a parse tree produced by auraParser#ListComp.
    def visitListComp(self, ctx:auraParser.ListCompContext):
        return self.visitChildren(ctx)


    # Visit a parse tree produced by auraParser#SetComp.
    def visitSetComp(self, ctx:auraParser.SetCompContext):
        return self.visitChildren(ctx)


    # Visit a parse tree produced by auraParser#comprehensionClause.
    def visitComprehensionClause(self, ctx:auraParser.ComprehensionClauseContext):
        return self.visitChildren(ctx)


    # Visit a parse tree produced by auraParser#matchExpr.
    def visitMatchExpr(self, ctx:auraParser.MatchExprContext):
        return self.visitChildren(ctx)


    # Visit a parse tree produced by auraParser#blockExpr.
    def visitBlockExpr(self, ctx:auraParser.BlockExprContext):
        return self.visitChildren(ctx)


    # Visit a parse tree produced by auraParser#pattern.
    def visitPattern(self, ctx:auraParser.PatternContext):
        return self.visitChildren(ctx)


    # Visit a parse tree produced by auraParser#orPattern.
    def visitOrPattern(self, ctx:auraParser.OrPatternContext):
        return self.visitChildren(ctx)


    # Visit a parse tree produced by auraParser#asPattern.
    def visitAsPattern(self, ctx:auraParser.AsPatternContext):
        return self.visitChildren(ctx)


    # Visit a parse tree produced by auraParser#ConstructorPattern.
    def visitConstructorPattern(self, ctx:auraParser.ConstructorPatternContext):
        return self.visitChildren(ctx)


    # Visit a parse tree produced by auraParser#ListPattern.
    def visitListPattern(self, ctx:auraParser.ListPatternContext):
        return self.visitChildren(ctx)


    # Visit a parse tree produced by auraParser#DictPattern.
    def visitDictPattern(self, ctx:auraParser.DictPatternContext):
        return self.visitChildren(ctx)


    # Visit a parse tree produced by auraParser#LiteralPattern.
    def visitLiteralPattern(self, ctx:auraParser.LiteralPatternContext):
        return self.visitChildren(ctx)


    # Visit a parse tree produced by auraParser#IdentifierPattern.
    def visitIdentifierPattern(self, ctx:auraParser.IdentifierPatternContext):
        return self.visitChildren(ctx)


    # Visit a parse tree produced by auraParser#WildcardPattern.
    def visitWildcardPattern(self, ctx:auraParser.WildcardPatternContext):
        return self.visitChildren(ctx)


    # Visit a parse tree produced by auraParser#patternList.
    def visitPatternList(self, ctx:auraParser.PatternListContext):
        return self.visitChildren(ctx)


    # Visit a parse tree produced by auraParser#fieldPatterns.
    def visitFieldPatterns(self, ctx:auraParser.FieldPatternsContext):
        return self.visitChildren(ctx)


    # Visit a parse tree produced by auraParser#fieldPattern.
    def visitFieldPattern(self, ctx:auraParser.FieldPatternContext):
        return self.visitChildren(ctx)


    # Visit a parse tree produced by auraParser#typeAnnotation.
    def visitTypeAnnotation(self, ctx:auraParser.TypeAnnotationContext):
        return self.visitChildren(ctx)


    # Visit a parse tree produced by auraParser#unionType.
    def visitUnionType(self, ctx:auraParser.UnionTypeContext):
        return self.visitChildren(ctx)


    # Visit a parse tree produced by auraParser#NamedOrGenericType.
    def visitNamedOrGenericType(self, ctx:auraParser.NamedOrGenericTypeContext):
        return self.visitChildren(ctx)


    # Visit a parse tree produced by auraParser#FunctionType.
    def visitFunctionType(self, ctx:auraParser.FunctionTypeContext):
        return self.visitChildren(ctx)


    # Visit a parse tree produced by auraParser#ListType.
    def visitListType(self, ctx:auraParser.ListTypeContext):
        return self.visitChildren(ctx)


    # Visit a parse tree produced by auraParser#DictType.
    def visitDictType(self, ctx:auraParser.DictTypeContext):
        return self.visitChildren(ctx)


    # Visit a parse tree produced by auraParser#StructuralType.
    def visitStructuralType(self, ctx:auraParser.StructuralTypeContext):
        return self.visitChildren(ctx)


    # Visit a parse tree produced by auraParser#typeArgs.
    def visitTypeArgs(self, ctx:auraParser.TypeArgsContext):
        return self.visitChildren(ctx)


    # Visit a parse tree produced by auraParser#fieldTypes.
    def visitFieldTypes(self, ctx:auraParser.FieldTypesContext):
        return self.visitChildren(ctx)


    # Visit a parse tree produced by auraParser#fieldType.
    def visitFieldType(self, ctx:auraParser.FieldTypeContext):
        return self.visitChildren(ctx)



del auraParser