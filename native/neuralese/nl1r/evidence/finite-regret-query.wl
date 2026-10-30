ExportString[Flatten[Table[{n,e,If[n==0,"NOT_APPLICABLE",ToString[(n-e)/n,InputForm]]},{n,{0,8,16}},{e,0,n}],1],"RawJSON"]
