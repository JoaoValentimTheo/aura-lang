# Generated from parser/aura.g4 by ANTLR 4.13.2
from antlr4 import *
if "." in __name__:
    from .auraParser import auraParser
else:
    from auraParser import auraParser

# This class defines a complete listener for a parse tree produced by auraParser.
class auraListener(ParseTreeListener):

    # Enter a parse tree produced by auraParser#program.
    def enterProgram(self, ctx:auraParser.ProgramContext):
        pass

    # Exit a parse tree produced by auraParser#program.
    def exitProgram(self, ctx:auraParser.ProgramContext):
        pass


    # Enter a parse tree produced by auraParser#ImportModule.
    def enterImportModule(self, ctx:auraParser.ImportModuleContext):
        pass

    # Exit a parse tree produced by auraParser#ImportModule.
    def exitImportModule(self, ctx:auraParser.ImportModuleContext):
        pass


    # Enter a parse tree produced by auraParser#FromImport.
    def enterFromImport(self, ctx:auraParser.FromImportContext):
        pass

    # Exit a parse tree produced by auraParser#FromImport.
    def exitFromImport(self, ctx:auraParser.FromImportContext):
        pass


    # Enter a parse tree produced by auraParser#modulePath.
    def enterModulePath(self, ctx:auraParser.ModulePathContext):
        pass

    # Exit a parse tree produced by auraParser#modulePath.
    def exitModulePath(self, ctx:auraParser.ModulePathContext):
        pass


    # Enter a parse tree produced by auraParser#importItems.
    def enterImportItems(self, ctx:auraParser.ImportItemsContext):
        pass

    # Exit a parse tree produced by auraParser#importItems.
    def exitImportItems(self, ctx:auraParser.ImportItemsContext):
        pass


    # Enter a parse tree produced by auraParser#declaration.
    def enterDeclaration(self, ctx:auraParser.DeclarationContext):
        pass

    # Exit a parse tree produced by auraParser#declaration.
    def exitDeclaration(self, ctx:auraParser.DeclarationContext):
        pass


    # Enter a parse tree produced by auraParser#varDecl.
    def enterVarDecl(self, ctx:auraParser.VarDeclContext):
        pass

    # Exit a parse tree produced by auraParser#varDecl.
    def exitVarDecl(self, ctx:auraParser.VarDeclContext):
        pass


    # Enter a parse tree produced by auraParser#constDecl.
    def enterConstDecl(self, ctx:auraParser.ConstDeclContext):
        pass

    # Exit a parse tree produced by auraParser#constDecl.
    def exitConstDecl(self, ctx:auraParser.ConstDeclContext):
        pass


    # Enter a parse tree produced by auraParser#funcDecl.
    def enterFuncDecl(self, ctx:auraParser.FuncDeclContext):
        pass

    # Exit a parse tree produced by auraParser#funcDecl.
    def exitFuncDecl(self, ctx:auraParser.FuncDeclContext):
        pass


    # Enter a parse tree produced by auraParser#typeParams.
    def enterTypeParams(self, ctx:auraParser.TypeParamsContext):
        pass

    # Exit a parse tree produced by auraParser#typeParams.
    def exitTypeParams(self, ctx:auraParser.TypeParamsContext):
        pass


    # Enter a parse tree produced by auraParser#parameterList.
    def enterParameterList(self, ctx:auraParser.ParameterListContext):
        pass

    # Exit a parse tree produced by auraParser#parameterList.
    def exitParameterList(self, ctx:auraParser.ParameterListContext):
        pass


    # Enter a parse tree produced by auraParser#PositionalParam.
    def enterPositionalParam(self, ctx:auraParser.PositionalParamContext):
        pass

    # Exit a parse tree produced by auraParser#PositionalParam.
    def exitPositionalParam(self, ctx:auraParser.PositionalParamContext):
        pass


    # Enter a parse tree produced by auraParser#VariadicParam.
    def enterVariadicParam(self, ctx:auraParser.VariadicParamContext):
        pass

    # Exit a parse tree produced by auraParser#VariadicParam.
    def exitVariadicParam(self, ctx:auraParser.VariadicParamContext):
        pass


    # Enter a parse tree produced by auraParser#KeywordOnlyMarker.
    def enterKeywordOnlyMarker(self, ctx:auraParser.KeywordOnlyMarkerContext):
        pass

    # Exit a parse tree produced by auraParser#KeywordOnlyMarker.
    def exitKeywordOnlyMarker(self, ctx:auraParser.KeywordOnlyMarkerContext):
        pass


    # Enter a parse tree produced by auraParser#KwargParam.
    def enterKwargParam(self, ctx:auraParser.KwargParamContext):
        pass

    # Exit a parse tree produced by auraParser#KwargParam.
    def exitKwargParam(self, ctx:auraParser.KwargParamContext):
        pass


    # Enter a parse tree produced by auraParser#returnType.
    def enterReturnType(self, ctx:auraParser.ReturnTypeContext):
        pass

    # Exit a parse tree produced by auraParser#returnType.
    def exitReturnType(self, ctx:auraParser.ReturnTypeContext):
        pass


    # Enter a parse tree produced by auraParser#classDecl.
    def enterClassDecl(self, ctx:auraParser.ClassDeclContext):
        pass

    # Exit a parse tree produced by auraParser#classDecl.
    def exitClassDecl(self, ctx:auraParser.ClassDeclContext):
        pass


    # Enter a parse tree produced by auraParser#FieldDef.
    def enterFieldDef(self, ctx:auraParser.FieldDefContext):
        pass

    # Exit a parse tree produced by auraParser#FieldDef.
    def exitFieldDef(self, ctx:auraParser.FieldDefContext):
        pass


    # Enter a parse tree produced by auraParser#ClassMethodDef.
    def enterClassMethodDef(self, ctx:auraParser.ClassMethodDefContext):
        pass

    # Exit a parse tree produced by auraParser#ClassMethodDef.
    def exitClassMethodDef(self, ctx:auraParser.ClassMethodDefContext):
        pass


    # Enter a parse tree produced by auraParser#ClassPropertyDef.
    def enterClassPropertyDef(self, ctx:auraParser.ClassPropertyDefContext):
        pass

    # Exit a parse tree produced by auraParser#ClassPropertyDef.
    def exitClassPropertyDef(self, ctx:auraParser.ClassPropertyDefContext):
        pass


    # Enter a parse tree produced by auraParser#methodDef.
    def enterMethodDef(self, ctx:auraParser.MethodDefContext):
        pass

    # Exit a parse tree produced by auraParser#methodDef.
    def exitMethodDef(self, ctx:auraParser.MethodDefContext):
        pass


    # Enter a parse tree produced by auraParser#propertyDef.
    def enterPropertyDef(self, ctx:auraParser.PropertyDefContext):
        pass

    # Exit a parse tree produced by auraParser#propertyDef.
    def exitPropertyDef(self, ctx:auraParser.PropertyDefContext):
        pass


    # Enter a parse tree produced by auraParser#traitDecl.
    def enterTraitDecl(self, ctx:auraParser.TraitDeclContext):
        pass

    # Exit a parse tree produced by auraParser#traitDecl.
    def exitTraitDecl(self, ctx:auraParser.TraitDeclContext):
        pass


    # Enter a parse tree produced by auraParser#traitMember.
    def enterTraitMember(self, ctx:auraParser.TraitMemberContext):
        pass

    # Exit a parse tree produced by auraParser#traitMember.
    def exitTraitMember(self, ctx:auraParser.TraitMemberContext):
        pass


    # Enter a parse tree produced by auraParser#typeDecl.
    def enterTypeDecl(self, ctx:auraParser.TypeDeclContext):
        pass

    # Exit a parse tree produced by auraParser#typeDecl.
    def exitTypeDecl(self, ctx:auraParser.TypeDeclContext):
        pass


    # Enter a parse tree produced by auraParser#moduleDecl.
    def enterModuleDecl(self, ctx:auraParser.ModuleDeclContext):
        pass

    # Exit a parse tree produced by auraParser#moduleDecl.
    def exitModuleDecl(self, ctx:auraParser.ModuleDeclContext):
        pass


    # Enter a parse tree produced by auraParser#moduleMember.
    def enterModuleMember(self, ctx:auraParser.ModuleMemberContext):
        pass

    # Exit a parse tree produced by auraParser#moduleMember.
    def exitModuleMember(self, ctx:auraParser.ModuleMemberContext):
        pass


    # Enter a parse tree produced by auraParser#decorator.
    def enterDecorator(self, ctx:auraParser.DecoratorContext):
        pass

    # Exit a parse tree produced by auraParser#decorator.
    def exitDecorator(self, ctx:auraParser.DecoratorContext):
        pass


    # Enter a parse tree produced by auraParser#statement.
    def enterStatement(self, ctx:auraParser.StatementContext):
        pass

    # Exit a parse tree produced by auraParser#statement.
    def exitStatement(self, ctx:auraParser.StatementContext):
        pass


    # Enter a parse tree produced by auraParser#assertStmt.
    def enterAssertStmt(self, ctx:auraParser.AssertStmtContext):
        pass

    # Exit a parse tree produced by auraParser#assertStmt.
    def exitAssertStmt(self, ctx:auraParser.AssertStmtContext):
        pass


    # Enter a parse tree produced by auraParser#exprStmt.
    def enterExprStmt(self, ctx:auraParser.ExprStmtContext):
        pass

    # Exit a parse tree produced by auraParser#exprStmt.
    def exitExprStmt(self, ctx:auraParser.ExprStmtContext):
        pass


    # Enter a parse tree produced by auraParser#ifStmt.
    def enterIfStmt(self, ctx:auraParser.IfStmtContext):
        pass

    # Exit a parse tree produced by auraParser#ifStmt.
    def exitIfStmt(self, ctx:auraParser.IfStmtContext):
        pass


    # Enter a parse tree produced by auraParser#unlessStmt.
    def enterUnlessStmt(self, ctx:auraParser.UnlessStmtContext):
        pass

    # Exit a parse tree produced by auraParser#unlessStmt.
    def exitUnlessStmt(self, ctx:auraParser.UnlessStmtContext):
        pass


    # Enter a parse tree produced by auraParser#guardStmt.
    def enterGuardStmt(self, ctx:auraParser.GuardStmtContext):
        pass

    # Exit a parse tree produced by auraParser#guardStmt.
    def exitGuardStmt(self, ctx:auraParser.GuardStmtContext):
        pass


    # Enter a parse tree produced by auraParser#whileStmt.
    def enterWhileStmt(self, ctx:auraParser.WhileStmtContext):
        pass

    # Exit a parse tree produced by auraParser#whileStmt.
    def exitWhileStmt(self, ctx:auraParser.WhileStmtContext):
        pass


    # Enter a parse tree produced by auraParser#untilStmt.
    def enterUntilStmt(self, ctx:auraParser.UntilStmtContext):
        pass

    # Exit a parse tree produced by auraParser#untilStmt.
    def exitUntilStmt(self, ctx:auraParser.UntilStmtContext):
        pass


    # Enter a parse tree produced by auraParser#forStmt.
    def enterForStmt(self, ctx:auraParser.ForStmtContext):
        pass

    # Exit a parse tree produced by auraParser#forStmt.
    def exitForStmt(self, ctx:auraParser.ForStmtContext):
        pass


    # Enter a parse tree produced by auraParser#loopStmt.
    def enterLoopStmt(self, ctx:auraParser.LoopStmtContext):
        pass

    # Exit a parse tree produced by auraParser#loopStmt.
    def exitLoopStmt(self, ctx:auraParser.LoopStmtContext):
        pass


    # Enter a parse tree produced by auraParser#breakStmt.
    def enterBreakStmt(self, ctx:auraParser.BreakStmtContext):
        pass

    # Exit a parse tree produced by auraParser#breakStmt.
    def exitBreakStmt(self, ctx:auraParser.BreakStmtContext):
        pass


    # Enter a parse tree produced by auraParser#continueStmt.
    def enterContinueStmt(self, ctx:auraParser.ContinueStmtContext):
        pass

    # Exit a parse tree produced by auraParser#continueStmt.
    def exitContinueStmt(self, ctx:auraParser.ContinueStmtContext):
        pass


    # Enter a parse tree produced by auraParser#returnStmt.
    def enterReturnStmt(self, ctx:auraParser.ReturnStmtContext):
        pass

    # Exit a parse tree produced by auraParser#returnStmt.
    def exitReturnStmt(self, ctx:auraParser.ReturnStmtContext):
        pass


    # Enter a parse tree produced by auraParser#tryStmt.
    def enterTryStmt(self, ctx:auraParser.TryStmtContext):
        pass

    # Exit a parse tree produced by auraParser#tryStmt.
    def exitTryStmt(self, ctx:auraParser.TryStmtContext):
        pass


    # Enter a parse tree produced by auraParser#exceptionPattern.
    def enterExceptionPattern(self, ctx:auraParser.ExceptionPatternContext):
        pass

    # Exit a parse tree produced by auraParser#exceptionPattern.
    def exitExceptionPattern(self, ctx:auraParser.ExceptionPatternContext):
        pass


    # Enter a parse tree produced by auraParser#withStmt.
    def enterWithStmt(self, ctx:auraParser.WithStmtContext):
        pass

    # Exit a parse tree produced by auraParser#withStmt.
    def exitWithStmt(self, ctx:auraParser.WithStmtContext):
        pass


    # Enter a parse tree produced by auraParser#withItem.
    def enterWithItem(self, ctx:auraParser.WithItemContext):
        pass

    # Exit a parse tree produced by auraParser#withItem.
    def exitWithItem(self, ctx:auraParser.WithItemContext):
        pass


    # Enter a parse tree produced by auraParser#matchStmt.
    def enterMatchStmt(self, ctx:auraParser.MatchStmtContext):
        pass

    # Exit a parse tree produced by auraParser#matchStmt.
    def exitMatchStmt(self, ctx:auraParser.MatchStmtContext):
        pass


    # Enter a parse tree produced by auraParser#matchCase.
    def enterMatchCase(self, ctx:auraParser.MatchCaseContext):
        pass

    # Exit a parse tree produced by auraParser#matchCase.
    def exitMatchCase(self, ctx:auraParser.MatchCaseContext):
        pass


    # Enter a parse tree produced by auraParser#blockStmt.
    def enterBlockStmt(self, ctx:auraParser.BlockStmtContext):
        pass

    # Exit a parse tree produced by auraParser#blockStmt.
    def exitBlockStmt(self, ctx:auraParser.BlockStmtContext):
        pass


    # Enter a parse tree produced by auraParser#expression.
    def enterExpression(self, ctx:auraParser.ExpressionContext):
        pass

    # Exit a parse tree produced by auraParser#expression.
    def exitExpression(self, ctx:auraParser.ExpressionContext):
        pass


    # Enter a parse tree produced by auraParser#assignment.
    def enterAssignment(self, ctx:auraParser.AssignmentContext):
        pass

    # Exit a parse tree produced by auraParser#assignment.
    def exitAssignment(self, ctx:auraParser.AssignmentContext):
        pass


    # Enter a parse tree produced by auraParser#assignOp.
    def enterAssignOp(self, ctx:auraParser.AssignOpContext):
        pass

    # Exit a parse tree produced by auraParser#assignOp.
    def exitAssignOp(self, ctx:auraParser.AssignOpContext):
        pass


    # Enter a parse tree produced by auraParser#pipe.
    def enterPipe(self, ctx:auraParser.PipeContext):
        pass

    # Exit a parse tree produced by auraParser#pipe.
    def exitPipe(self, ctx:auraParser.PipeContext):
        pass


    # Enter a parse tree produced by auraParser#ternary.
    def enterTernary(self, ctx:auraParser.TernaryContext):
        pass

    # Exit a parse tree produced by auraParser#ternary.
    def exitTernary(self, ctx:auraParser.TernaryContext):
        pass


    # Enter a parse tree produced by auraParser#elvis.
    def enterElvis(self, ctx:auraParser.ElvisContext):
        pass

    # Exit a parse tree produced by auraParser#elvis.
    def exitElvis(self, ctx:auraParser.ElvisContext):
        pass


    # Enter a parse tree produced by auraParser#coalesce.
    def enterCoalesce(self, ctx:auraParser.CoalesceContext):
        pass

    # Exit a parse tree produced by auraParser#coalesce.
    def exitCoalesce(self, ctx:auraParser.CoalesceContext):
        pass


    # Enter a parse tree produced by auraParser#logicalOr.
    def enterLogicalOr(self, ctx:auraParser.LogicalOrContext):
        pass

    # Exit a parse tree produced by auraParser#logicalOr.
    def exitLogicalOr(self, ctx:auraParser.LogicalOrContext):
        pass


    # Enter a parse tree produced by auraParser#logicalAnd.
    def enterLogicalAnd(self, ctx:auraParser.LogicalAndContext):
        pass

    # Exit a parse tree produced by auraParser#logicalAnd.
    def exitLogicalAnd(self, ctx:auraParser.LogicalAndContext):
        pass


    # Enter a parse tree produced by auraParser#bitwiseOr.
    def enterBitwiseOr(self, ctx:auraParser.BitwiseOrContext):
        pass

    # Exit a parse tree produced by auraParser#bitwiseOr.
    def exitBitwiseOr(self, ctx:auraParser.BitwiseOrContext):
        pass


    # Enter a parse tree produced by auraParser#bitwiseXor.
    def enterBitwiseXor(self, ctx:auraParser.BitwiseXorContext):
        pass

    # Exit a parse tree produced by auraParser#bitwiseXor.
    def exitBitwiseXor(self, ctx:auraParser.BitwiseXorContext):
        pass


    # Enter a parse tree produced by auraParser#bitwiseAnd.
    def enterBitwiseAnd(self, ctx:auraParser.BitwiseAndContext):
        pass

    # Exit a parse tree produced by auraParser#bitwiseAnd.
    def exitBitwiseAnd(self, ctx:auraParser.BitwiseAndContext):
        pass


    # Enter a parse tree produced by auraParser#equality.
    def enterEquality(self, ctx:auraParser.EqualityContext):
        pass

    # Exit a parse tree produced by auraParser#equality.
    def exitEquality(self, ctx:auraParser.EqualityContext):
        pass


    # Enter a parse tree produced by auraParser#comparison.
    def enterComparison(self, ctx:auraParser.ComparisonContext):
        pass

    # Exit a parse tree produced by auraParser#comparison.
    def exitComparison(self, ctx:auraParser.ComparisonContext):
        pass


    # Enter a parse tree produced by auraParser#InclusiveRange.
    def enterInclusiveRange(self, ctx:auraParser.InclusiveRangeContext):
        pass

    # Exit a parse tree produced by auraParser#InclusiveRange.
    def exitInclusiveRange(self, ctx:auraParser.InclusiveRangeContext):
        pass


    # Enter a parse tree produced by auraParser#ExclusiveRange.
    def enterExclusiveRange(self, ctx:auraParser.ExclusiveRangeContext):
        pass

    # Exit a parse tree produced by auraParser#ExclusiveRange.
    def exitExclusiveRange(self, ctx:auraParser.ExclusiveRangeContext):
        pass


    # Enter a parse tree produced by auraParser#InfiniteRange.
    def enterInfiniteRange(self, ctx:auraParser.InfiniteRangeContext):
        pass

    # Exit a parse tree produced by auraParser#InfiniteRange.
    def exitInfiniteRange(self, ctx:auraParser.InfiniteRangeContext):
        pass


    # Enter a parse tree produced by auraParser#RangeBase.
    def enterRangeBase(self, ctx:auraParser.RangeBaseContext):
        pass

    # Exit a parse tree produced by auraParser#RangeBase.
    def exitRangeBase(self, ctx:auraParser.RangeBaseContext):
        pass


    # Enter a parse tree produced by auraParser#shift.
    def enterShift(self, ctx:auraParser.ShiftContext):
        pass

    # Exit a parse tree produced by auraParser#shift.
    def exitShift(self, ctx:auraParser.ShiftContext):
        pass


    # Enter a parse tree produced by auraParser#additive.
    def enterAdditive(self, ctx:auraParser.AdditiveContext):
        pass

    # Exit a parse tree produced by auraParser#additive.
    def exitAdditive(self, ctx:auraParser.AdditiveContext):
        pass


    # Enter a parse tree produced by auraParser#multiplicative.
    def enterMultiplicative(self, ctx:auraParser.MultiplicativeContext):
        pass

    # Exit a parse tree produced by auraParser#multiplicative.
    def exitMultiplicative(self, ctx:auraParser.MultiplicativeContext):
        pass


    # Enter a parse tree produced by auraParser#castExpr.
    def enterCastExpr(self, ctx:auraParser.CastExprContext):
        pass

    # Exit a parse tree produced by auraParser#castExpr.
    def exitCastExpr(self, ctx:auraParser.CastExprContext):
        pass


    # Enter a parse tree produced by auraParser#unary.
    def enterUnary(self, ctx:auraParser.UnaryContext):
        pass

    # Exit a parse tree produced by auraParser#unary.
    def exitUnary(self, ctx:auraParser.UnaryContext):
        pass


    # Enter a parse tree produced by auraParser#exponentiation.
    def enterExponentiation(self, ctx:auraParser.ExponentiationContext):
        pass

    # Exit a parse tree produced by auraParser#exponentiation.
    def exitExponentiation(self, ctx:auraParser.ExponentiationContext):
        pass


    # Enter a parse tree produced by auraParser#postfix.
    def enterPostfix(self, ctx:auraParser.PostfixContext):
        pass

    # Exit a parse tree produced by auraParser#postfix.
    def exitPostfix(self, ctx:auraParser.PostfixContext):
        pass


    # Enter a parse tree produced by auraParser#call.
    def enterCall(self, ctx:auraParser.CallContext):
        pass

    # Exit a parse tree produced by auraParser#call.
    def exitCall(self, ctx:auraParser.CallContext):
        pass


    # Enter a parse tree produced by auraParser#index.
    def enterIndex(self, ctx:auraParser.IndexContext):
        pass

    # Exit a parse tree produced by auraParser#index.
    def exitIndex(self, ctx:auraParser.IndexContext):
        pass


    # Enter a parse tree produced by auraParser#member.
    def enterMember(self, ctx:auraParser.MemberContext):
        pass

    # Exit a parse tree produced by auraParser#member.
    def exitMember(self, ctx:auraParser.MemberContext):
        pass


    # Enter a parse tree produced by auraParser#safeNav.
    def enterSafeNav(self, ctx:auraParser.SafeNavContext):
        pass

    # Exit a parse tree produced by auraParser#safeNav.
    def exitSafeNav(self, ctx:auraParser.SafeNavContext):
        pass


    # Enter a parse tree produced by auraParser#spreadOp.
    def enterSpreadOp(self, ctx:auraParser.SpreadOpContext):
        pass

    # Exit a parse tree produced by auraParser#spreadOp.
    def exitSpreadOp(self, ctx:auraParser.SpreadOpContext):
        pass


    # Enter a parse tree produced by auraParser#primary.
    def enterPrimary(self, ctx:auraParser.PrimaryContext):
        pass

    # Exit a parse tree produced by auraParser#primary.
    def exitPrimary(self, ctx:auraParser.PrimaryContext):
        pass


    # Enter a parse tree produced by auraParser#literal.
    def enterLiteral(self, ctx:auraParser.LiteralContext):
        pass

    # Exit a parse tree produced by auraParser#literal.
    def exitLiteral(self, ctx:auraParser.LiteralContext):
        pass


    # Enter a parse tree produced by auraParser#argumentList.
    def enterArgumentList(self, ctx:auraParser.ArgumentListContext):
        pass

    # Exit a parse tree produced by auraParser#argumentList.
    def exitArgumentList(self, ctx:auraParser.ArgumentListContext):
        pass


    # Enter a parse tree produced by auraParser#PositionalArg.
    def enterPositionalArg(self, ctx:auraParser.PositionalArgContext):
        pass

    # Exit a parse tree produced by auraParser#PositionalArg.
    def exitPositionalArg(self, ctx:auraParser.PositionalArgContext):
        pass


    # Enter a parse tree produced by auraParser#KeywordArg.
    def enterKeywordArg(self, ctx:auraParser.KeywordArgContext):
        pass

    # Exit a parse tree produced by auraParser#KeywordArg.
    def exitKeywordArg(self, ctx:auraParser.KeywordArgContext):
        pass


    # Enter a parse tree produced by auraParser#EmptyList.
    def enterEmptyList(self, ctx:auraParser.EmptyListContext):
        pass

    # Exit a parse tree produced by auraParser#EmptyList.
    def exitEmptyList(self, ctx:auraParser.EmptyListContext):
        pass


    # Enter a parse tree produced by auraParser#NonEmptyList.
    def enterNonEmptyList(self, ctx:auraParser.NonEmptyListContext):
        pass

    # Exit a parse tree produced by auraParser#NonEmptyList.
    def exitNonEmptyList(self, ctx:auraParser.NonEmptyListContext):
        pass


    # Enter a parse tree produced by auraParser#ListComprehension.
    def enterListComprehension(self, ctx:auraParser.ListComprehensionContext):
        pass

    # Exit a parse tree produced by auraParser#ListComprehension.
    def exitListComprehension(self, ctx:auraParser.ListComprehensionContext):
        pass


    # Enter a parse tree produced by auraParser#EmptyDict.
    def enterEmptyDict(self, ctx:auraParser.EmptyDictContext):
        pass

    # Exit a parse tree produced by auraParser#EmptyDict.
    def exitEmptyDict(self, ctx:auraParser.EmptyDictContext):
        pass


    # Enter a parse tree produced by auraParser#NonEmptyDict.
    def enterNonEmptyDict(self, ctx:auraParser.NonEmptyDictContext):
        pass

    # Exit a parse tree produced by auraParser#NonEmptyDict.
    def exitNonEmptyDict(self, ctx:auraParser.NonEmptyDictContext):
        pass


    # Enter a parse tree produced by auraParser#DictComp.
    def enterDictComp(self, ctx:auraParser.DictCompContext):
        pass

    # Exit a parse tree produced by auraParser#DictComp.
    def exitDictComp(self, ctx:auraParser.DictCompContext):
        pass


    # Enter a parse tree produced by auraParser#kvPair.
    def enterKvPair(self, ctx:auraParser.KvPairContext):
        pass

    # Exit a parse tree produced by auraParser#kvPair.
    def exitKvPair(self, ctx:auraParser.KvPairContext):
        pass


    # Enter a parse tree produced by auraParser#setLiteral.
    def enterSetLiteral(self, ctx:auraParser.SetLiteralContext):
        pass

    # Exit a parse tree produced by auraParser#setLiteral.
    def exitSetLiteral(self, ctx:auraParser.SetLiteralContext):
        pass


    # Enter a parse tree produced by auraParser#EmptyTuple.
    def enterEmptyTuple(self, ctx:auraParser.EmptyTupleContext):
        pass

    # Exit a parse tree produced by auraParser#EmptyTuple.
    def exitEmptyTuple(self, ctx:auraParser.EmptyTupleContext):
        pass


    # Enter a parse tree produced by auraParser#NonEmptyTuple.
    def enterNonEmptyTuple(self, ctx:auraParser.NonEmptyTupleContext):
        pass

    # Exit a parse tree produced by auraParser#NonEmptyTuple.
    def exitNonEmptyTuple(self, ctx:auraParser.NonEmptyTupleContext):
        pass


    # Enter a parse tree produced by auraParser#lambdaExpr.
    def enterLambdaExpr(self, ctx:auraParser.LambdaExprContext):
        pass

    # Exit a parse tree produced by auraParser#lambdaExpr.
    def exitLambdaExpr(self, ctx:auraParser.LambdaExprContext):
        pass


    # Enter a parse tree produced by auraParser#ListComp.
    def enterListComp(self, ctx:auraParser.ListCompContext):
        pass

    # Exit a parse tree produced by auraParser#ListComp.
    def exitListComp(self, ctx:auraParser.ListCompContext):
        pass


    # Enter a parse tree produced by auraParser#SetComp.
    def enterSetComp(self, ctx:auraParser.SetCompContext):
        pass

    # Exit a parse tree produced by auraParser#SetComp.
    def exitSetComp(self, ctx:auraParser.SetCompContext):
        pass


    # Enter a parse tree produced by auraParser#comprehensionClause.
    def enterComprehensionClause(self, ctx:auraParser.ComprehensionClauseContext):
        pass

    # Exit a parse tree produced by auraParser#comprehensionClause.
    def exitComprehensionClause(self, ctx:auraParser.ComprehensionClauseContext):
        pass


    # Enter a parse tree produced by auraParser#matchExpr.
    def enterMatchExpr(self, ctx:auraParser.MatchExprContext):
        pass

    # Exit a parse tree produced by auraParser#matchExpr.
    def exitMatchExpr(self, ctx:auraParser.MatchExprContext):
        pass


    # Enter a parse tree produced by auraParser#blockExpr.
    def enterBlockExpr(self, ctx:auraParser.BlockExprContext):
        pass

    # Exit a parse tree produced by auraParser#blockExpr.
    def exitBlockExpr(self, ctx:auraParser.BlockExprContext):
        pass


    # Enter a parse tree produced by auraParser#pattern.
    def enterPattern(self, ctx:auraParser.PatternContext):
        pass

    # Exit a parse tree produced by auraParser#pattern.
    def exitPattern(self, ctx:auraParser.PatternContext):
        pass


    # Enter a parse tree produced by auraParser#orPattern.
    def enterOrPattern(self, ctx:auraParser.OrPatternContext):
        pass

    # Exit a parse tree produced by auraParser#orPattern.
    def exitOrPattern(self, ctx:auraParser.OrPatternContext):
        pass


    # Enter a parse tree produced by auraParser#asPattern.
    def enterAsPattern(self, ctx:auraParser.AsPatternContext):
        pass

    # Exit a parse tree produced by auraParser#asPattern.
    def exitAsPattern(self, ctx:auraParser.AsPatternContext):
        pass


    # Enter a parse tree produced by auraParser#ConstructorPattern.
    def enterConstructorPattern(self, ctx:auraParser.ConstructorPatternContext):
        pass

    # Exit a parse tree produced by auraParser#ConstructorPattern.
    def exitConstructorPattern(self, ctx:auraParser.ConstructorPatternContext):
        pass


    # Enter a parse tree produced by auraParser#ListPattern.
    def enterListPattern(self, ctx:auraParser.ListPatternContext):
        pass

    # Exit a parse tree produced by auraParser#ListPattern.
    def exitListPattern(self, ctx:auraParser.ListPatternContext):
        pass


    # Enter a parse tree produced by auraParser#DictPattern.
    def enterDictPattern(self, ctx:auraParser.DictPatternContext):
        pass

    # Exit a parse tree produced by auraParser#DictPattern.
    def exitDictPattern(self, ctx:auraParser.DictPatternContext):
        pass


    # Enter a parse tree produced by auraParser#LiteralPattern.
    def enterLiteralPattern(self, ctx:auraParser.LiteralPatternContext):
        pass

    # Exit a parse tree produced by auraParser#LiteralPattern.
    def exitLiteralPattern(self, ctx:auraParser.LiteralPatternContext):
        pass


    # Enter a parse tree produced by auraParser#IdentifierPattern.
    def enterIdentifierPattern(self, ctx:auraParser.IdentifierPatternContext):
        pass

    # Exit a parse tree produced by auraParser#IdentifierPattern.
    def exitIdentifierPattern(self, ctx:auraParser.IdentifierPatternContext):
        pass


    # Enter a parse tree produced by auraParser#WildcardPattern.
    def enterWildcardPattern(self, ctx:auraParser.WildcardPatternContext):
        pass

    # Exit a parse tree produced by auraParser#WildcardPattern.
    def exitWildcardPattern(self, ctx:auraParser.WildcardPatternContext):
        pass


    # Enter a parse tree produced by auraParser#patternList.
    def enterPatternList(self, ctx:auraParser.PatternListContext):
        pass

    # Exit a parse tree produced by auraParser#patternList.
    def exitPatternList(self, ctx:auraParser.PatternListContext):
        pass


    # Enter a parse tree produced by auraParser#fieldPatterns.
    def enterFieldPatterns(self, ctx:auraParser.FieldPatternsContext):
        pass

    # Exit a parse tree produced by auraParser#fieldPatterns.
    def exitFieldPatterns(self, ctx:auraParser.FieldPatternsContext):
        pass


    # Enter a parse tree produced by auraParser#fieldPattern.
    def enterFieldPattern(self, ctx:auraParser.FieldPatternContext):
        pass

    # Exit a parse tree produced by auraParser#fieldPattern.
    def exitFieldPattern(self, ctx:auraParser.FieldPatternContext):
        pass


    # Enter a parse tree produced by auraParser#typeAnnotation.
    def enterTypeAnnotation(self, ctx:auraParser.TypeAnnotationContext):
        pass

    # Exit a parse tree produced by auraParser#typeAnnotation.
    def exitTypeAnnotation(self, ctx:auraParser.TypeAnnotationContext):
        pass


    # Enter a parse tree produced by auraParser#unionType.
    def enterUnionType(self, ctx:auraParser.UnionTypeContext):
        pass

    # Exit a parse tree produced by auraParser#unionType.
    def exitUnionType(self, ctx:auraParser.UnionTypeContext):
        pass


    # Enter a parse tree produced by auraParser#NamedOrGenericType.
    def enterNamedOrGenericType(self, ctx:auraParser.NamedOrGenericTypeContext):
        pass

    # Exit a parse tree produced by auraParser#NamedOrGenericType.
    def exitNamedOrGenericType(self, ctx:auraParser.NamedOrGenericTypeContext):
        pass


    # Enter a parse tree produced by auraParser#FunctionType.
    def enterFunctionType(self, ctx:auraParser.FunctionTypeContext):
        pass

    # Exit a parse tree produced by auraParser#FunctionType.
    def exitFunctionType(self, ctx:auraParser.FunctionTypeContext):
        pass


    # Enter a parse tree produced by auraParser#ListType.
    def enterListType(self, ctx:auraParser.ListTypeContext):
        pass

    # Exit a parse tree produced by auraParser#ListType.
    def exitListType(self, ctx:auraParser.ListTypeContext):
        pass


    # Enter a parse tree produced by auraParser#DictType.
    def enterDictType(self, ctx:auraParser.DictTypeContext):
        pass

    # Exit a parse tree produced by auraParser#DictType.
    def exitDictType(self, ctx:auraParser.DictTypeContext):
        pass


    # Enter a parse tree produced by auraParser#StructuralType.
    def enterStructuralType(self, ctx:auraParser.StructuralTypeContext):
        pass

    # Exit a parse tree produced by auraParser#StructuralType.
    def exitStructuralType(self, ctx:auraParser.StructuralTypeContext):
        pass


    # Enter a parse tree produced by auraParser#typeArgs.
    def enterTypeArgs(self, ctx:auraParser.TypeArgsContext):
        pass

    # Exit a parse tree produced by auraParser#typeArgs.
    def exitTypeArgs(self, ctx:auraParser.TypeArgsContext):
        pass


    # Enter a parse tree produced by auraParser#fieldTypes.
    def enterFieldTypes(self, ctx:auraParser.FieldTypesContext):
        pass

    # Exit a parse tree produced by auraParser#fieldTypes.
    def exitFieldTypes(self, ctx:auraParser.FieldTypesContext):
        pass


    # Enter a parse tree produced by auraParser#fieldType.
    def enterFieldType(self, ctx:auraParser.FieldTypeContext):
        pass

    # Exit a parse tree produced by auraParser#fieldType.
    def exitFieldType(self, ctx:auraParser.FieldTypeContext):
        pass



del auraParser