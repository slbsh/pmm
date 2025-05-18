(def COLOUR [255 165 0])

(defn parse-pkg [crate] 
	(let [owners (->> ((crate :links) :owners)
						   (string "https://crates.io")
						   (get-req) (json->janet))]
		{ :name (crate :name)
		  :version (crate :default_version)
		  :description (crate :description)
		  :url (string "https://crates.io/crates/" (crate :name))
		  :authors (map |(string ($ :login) " (" ($ :name) ")") (owners :users)) }
	)
)

(defn parse-time [time]
	(let [[date time] (string/split "T" time)
			[time _]    (string/split "." time)]
		(string date " " time))
)

(defn search [name]
	(map parse-pkg 
		(-> (string "https://crates.io/api/v1/crates?q=" name)
			 (get-req) (json->janet) (get :crates)))
)

(defn info [& name]
	(let [name (string/join "%20" (apply (tuple) name))
			json (-> (string "https://crates.io/api/v1/crates/" name)
						(get-req) (json->janet))
		  _ (if (not (nil? (json :errors)))
			  (error (-> json (get :errors) (get 0) (get :detail))))
		  crate (json :crate)
		  recent (find |(= ($ :num) (crate :default_version)) (json :versions)) ]
		{ :pkg (parse-pkg crate)
		  :deps @["TODO"]
		  :homepage     (crate :homepage)
		  :source       (crate :repository)
		  :groups       (tuple/join (crate :categories) (crate :keywords))
		  :downloads    (crate :downloads)
		  :license      (recent :license)
		  :release-date (parse-time (recent :created_at))
		  :size         (recent :crate_size) }
	)
)

(defn add [pkgs]
	()
)

(defn test [] 
	(def libc (ffi/native "/lib/libc.so.6"))
	(def puts (ffi-func libc "puts" :int :string))

	(puts "Hello, world!")

	(error "This is a test error message." )
)
