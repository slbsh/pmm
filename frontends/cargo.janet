(defn list [& args]
	(def out (exec "cargo" "search" (first args)))

	(->>
		(let [l (string/split "\n" (out :stdout))] 
			  (slice l 0 (- (length l) 2)))
		(map (fn [line]
			(def line (string/split " " line))

			{
				:name (first line)
				:version (let [v (get line 2)]
								(slice v 1 (- (length v) 1)))
			}
		))
	)
)

(defn query [& args] (do
	(each arg args (print arg))
	(first args)
))
