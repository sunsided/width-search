begin_version
3
end_version
begin_metric
0
end_metric
6
begin_variable
var0
-1
3
Atom on(c, a)
Atom on(c, b)
Atom on(c, table)
end_variable
begin_variable
var1
-1
2
Atom clear(a)
NegatedAtom clear(a)
end_variable
begin_variable
var2
-1
2
Atom clear(b)
NegatedAtom clear(b)
end_variable
begin_variable
var3
-1
2
Atom clear(c)
NegatedAtom clear(c)
end_variable
begin_variable
var4
-1
3
Atom on(a, b)
Atom on(a, c)
Atom on(a, table)
end_variable
begin_variable
var5
-1
3
Atom on(b, a)
Atom on(b, c)
Atom on(b, table)
end_variable
0
begin_state
0
1
0
0
2
2
end_state
begin_goal
2
4 0
5 1
end_goal
18
begin_operator
puton a b c
1
1 0
3
0 2 0 1
0 3 -1 0
0 4 1 0
1
end_operator
begin_operator
puton a b table
1
1 0
2
0 2 0 1
0 4 2 0
1
end_operator
begin_operator
puton a c b
1
1 0
3
0 2 -1 0
0 3 0 1
0 4 0 1
1
end_operator
begin_operator
puton a c table
1
1 0
2
0 3 0 1
0 4 2 1
1
end_operator
begin_operator
puton a table b
1
1 0
2
0 2 -1 0
0 4 0 2
1
end_operator
begin_operator
puton a table c
1
1 0
2
0 3 -1 0
0 4 1 2
1
end_operator
begin_operator
puton b a c
1
2 0
3
0 1 0 1
0 3 -1 0
0 5 1 0
1
end_operator
begin_operator
puton b a table
1
2 0
2
0 1 0 1
0 5 2 0
1
end_operator
begin_operator
puton b c a
1
2 0
3
0 1 -1 0
0 3 0 1
0 5 0 1
1
end_operator
begin_operator
puton b c table
1
2 0
2
0 3 0 1
0 5 2 1
1
end_operator
begin_operator
puton b table a
1
2 0
2
0 1 -1 0
0 5 0 2
1
end_operator
begin_operator
puton b table c
1
2 0
2
0 3 -1 0
0 5 1 2
1
end_operator
begin_operator
puton c a b
1
3 0
3
0 1 0 1
0 2 -1 0
0 0 1 0
1
end_operator
begin_operator
puton c a table
1
3 0
2
0 1 0 1
0 0 2 0
1
end_operator
begin_operator
puton c b a
1
3 0
3
0 1 -1 0
0 2 0 1
0 0 0 1
1
end_operator
begin_operator
puton c b table
1
3 0
2
0 2 0 1
0 0 2 1
1
end_operator
begin_operator
puton c table a
1
3 0
2
0 1 -1 0
0 0 0 2
1
end_operator
begin_operator
puton c table b
1
3 0
2
0 2 -1 0
0 0 1 2
1
end_operator
0
