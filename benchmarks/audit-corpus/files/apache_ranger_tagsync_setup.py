# Source: https://github.com/apache/ranger/blob/a61d9c08e36c753078c3b00aea790b117d6a20d0/tagsync/scripts/setup.py
# License: Apache-2.0
# Note: minimal extract from apache/ranger@a61d9c08 tagsync/scripts/setup.py (upstream 527 lines, Apache-2.0). Batch 10 density-support file with `expected: []`: pr-miner mining-DB density. The upstream file contains three top-level defs that explicitly pair open() with close() — `convertInstallPropsToXML` (upstream line 219), `write_env_files` (upstream line 325), and `main` (upstream line 333) — plus three open-only top-level defs (`populate_global_dict` 105, `getPropertiesConfigMap` 148, `getPropertiesKeyList` 161), all of which a verbatim copy would contribute to the mining database as count(open) without count(close), lowering the {open} -> {close} confidence ratio. This minimal extract therefore keeps only the three paired defs plus the upstream imports they reference, so the file parses cleanly under tree-sitter Python 3. The Semgrep `open-never-closed` rule produces no findings on these three defs (each opens AND explicitly closes the file handle within the same function body). pr-miner's spec F2 extracts items including `{open, close, write, ...}` for each, contributing three paired open+close transactions to the mining database. The file's net pr-miner contribution is +3 to both numerator (open+close) and denominator (open) of the {open} -> {close} confidence ratio. SHA-256 is of the extracted file as committed (per benchmarks/audit-corpus/README.md "minimal extracts" clause).

# Imports retained for parseability of the extracted defs.
from io import StringIO
from configparser import ConfigParser
from urllib.parse import urlparse
import re
import xml.etree.ElementTree as ET
import os,errno,sys,getopt
from os import listdir
from os.path import isfile, join, dirname, basename
from time import gmtime, strftime, localtime
from xml import etree
import shutil
import pwd, grp


def convertInstallPropsToXML(props):
	directKeyMap = getPropertiesConfigMap(join(installTemplateDirName,install2xmlMapFileName))
	ret = {}
	atlasOutFn = join(confFolderName, atlasApplicationPropFileName)

	atlasOutFile = open(atlasOutFn, "w")

	atlas_principal = ''
	atlas_keytab = ''

	for k,v in props.items():
		if (k in list(directKeyMap)):
			newKey = directKeyMap[k]
			if (k == TAGSYNC_ATLAS_KAFKA_ENDPOINTS_KEY):
				atlasOutFile.write(newKey + "=" + v + "\n")
			elif (k == TAGSYNC_ATLAS_ZOOKEEPER_ENDPOINT_KEY):
				atlasOutFile.write(newKey + "=" + v + "\n")
			elif (k == TAGSYNC_ATLAS_KAFKA_OFFSETS_TOPIC_REPLICATION_FACTOR_KEY):
				atlasOutFile.write(newKey + "=" + v + "\n")
			elif (k == TAGSYNC_ATLAS_CONSUMER_GROUP_KEY):
				atlasOutFile.write(newKey + "=" + v + "\n")
			elif (configure_security and k == TAG_SOURCE_ATLAS_KAKFA_SERVICE_NAME_KEY):
				atlasOutFile.write(newKey + "=" + v + "\n")
			elif (configure_security and k == TAG_SOURCE_ATLAS_KAFKA_SECURITY_PROTOCOL_KEY):
				atlasOutFile.write(newKey + "=" + v + "\n")
			elif (configure_security and k == TAG_SOURCE_ATLAS_KERBEROS_PRINCIPAL_KEY):
				atlas_principal = v
			elif (configure_security and k == TAG_SOURCE_ATLAS_KERBEROS_KEYTAB_KEY):
				atlas_keytab = v
			else:
				ret[newKey] = v
		else:
			print("INFO: Direct Key not found:%s" % (k))

	if (configure_security):
		atlasOutFile.write("atlas.jaas.KafkaClient.loginModuleName = com.sun.security.auth.module.Krb5LoginModule" + "\n")
		atlasOutFile.write("atlas.jaas.KafkaClient.loginModuleControlFlag = required" + "\n")
		atlasOutFile.write("atlas.jaas.KafkaClient.option.useKeyTab = true" + "\n")
		atlasOutFile.write("atlas.jaas.KafkaClient.option.storeKey = true" + "\n")
		atlasOutFile.write("atlas.jaas.KafkaClient.option.serviceName = kafka" + "\n")
		atlasOutFile.write("atlas.jaas.KafkaClient.option.keyTab = " + atlas_keytab + "\n")
		atlasOutFile.write("atlas.jaas.KafkaClient.option.principal = " + atlas_principal + "\n")

	atlasOutFile.close()

	if (TAG_SOURCE_ATLAS_ENABLED_KEY in ret):
		ret[TAG_SOURCE_ATLAS_ENABLED] = ret[TAG_SOURCE_ATLAS_ENABLED_KEY]
		del ret[TAG_SOURCE_ATLAS_ENABLED_KEY]

	if (TAG_SOURCE_ATLASREST_ENABLED_KEY in ret):
		ret[TAG_SOURCE_ATLASREST_ENABLED] = ret[TAG_SOURCE_ATLASREST_ENABLED_KEY]
		del ret[TAG_SOURCE_ATLASREST_ENABLED_KEY]

	if (TAG_SOURCE_FILE_ENABLED_KEY in ret):
		ret[TAG_SOURCE_FILE_ENABLED] = ret[TAG_SOURCE_FILE_ENABLED_KEY]
		del ret[TAG_SOURCE_FILE_ENABLED_KEY]

	return ret


def write_env_files(exp_var_name, log_path, file_name):
        final_path = "{0}/{1}".format(confBaseDirName,file_name)
        if not os.path.isfile(final_path):
            print("INFO: Creating %s file" % file_name)
        f = open(final_path, "w")
        f.write("export {0}={1}".format(exp_var_name,log_path))
        f.close()


def main():

	global configure_security

	print("\nINFO: Installing ranger-tagsync .....\n")

	populate_global_dict()


	kerberize = globalDict['is_secure']
	if kerberize != "":
		kerberize = kerberize.lower()
		if kerberize == "true" or kerberize == "enabled" or kerberize == "yes":
			configure_security = True


	hadoop_conf = globalDict['hadoop_conf']
	pid_dir_path = globalDict['TAGSYNC_PID_DIR_PATH']
	unix_user = globalDict['unix_user']

	if pid_dir_path == "":
		pid_dir_path = "/var/run/ranger"

	dirList = [ rangerBaseDirName, tagsyncBaseDirFullName, confFolderName ]
	for dir in dirList:
		if (not os.path.isdir(dir)):
			os.makedirs(dir,0o755)

	defFileList = [ logbackFileName ]
	for defFile in defFileList:
		fn = join(confDistDirName, defFile)
		if ( isfile(fn) ):
			shutil.copy(fn,join(confFolderName,defFile))

	#
	# Create JAVA_HOME setting in confFolderName
	#
	java_home_setter_fn = join(confFolderName, 'java_home.sh')
	if isfile(java_home_setter_fn):
		archiveFile(java_home_setter_fn)
	jhf = open(java_home_setter_fn, 'w')
	str = "export JAVA_HOME=%s\n" % os.environ['JAVA_HOME']
	jhf.write(str)
	jhf.close()
	os.chmod(java_home_setter_fn,0o750)


	if (not os.path.isdir(localConfFolderName)):
		os.symlink(confFolderName, localConfFolderName)

	installProps = getPropertiesConfigMap(join(installPropDirName,installPropFileName))
	modifiedInstallProps = convertInstallPropsToXML(installProps)

	mergeProps = {}
	mergeProps.update(modifiedInstallProps)

	localLogFolderName = mergeProps['ranger.tagsync.logdir']
	if (not os.path.isdir(localLogFolderName)):
		if (localLogFolderName != tagsyncLogFolderName):
			os.symlink(tagsyncLogFolderName, localLogFolderName)

	fn = join(installTemplateDirName,templateFileName)
	outfn = join(confFolderName, outputFileName)

	if ( os.path.isdir(logFolderName) ):
		logStat = os.stat(logFolderName)
		logStat.st_uid
		logStat.st_gid
		ownerName = pwd.getpwuid(logStat.st_uid).pw_name
		groupName = pwd.getpwuid(logStat.st_uid).pw_name
	else:
		os.makedirs(logFolderName,logFolderPermMode)

	if (not os.path.isdir(tagsyncLogFolderName)):
		os.makedirs(tagsyncLogFolderName,logFolderPermMode)

	if (not os.path.isdir(pid_dir_path)):
		os.makedirs(pid_dir_path,logFolderPermMode)

	if (unixUserProp in mergeProps):
		ownerName = mergeProps[unixUserProp]
	else:
		mergeProps[unixUserProp] = "ranger"
		ownerName = mergeProps[unixUserProp]

	if (unixGroupProp in mergeProps):
		groupName = mergeProps[unixGroupProp]
	else:
		mergeProps[unixGroupProp] = "ranger"
		groupName = mergeProps[unixGroupProp]

	try:
		groupId = grp.getgrnam(groupName).gr_gid
	except KeyError as e:
		groupId = createGroup(groupName)

	try:
		ownerId = pwd.getpwnam(ownerName).pw_uid
	except KeyError as e:
		ownerId = createUser(ownerName, groupName)

	os.chown(logFolderName,ownerId,groupId)
	os.chown(tagsyncLogFolderName,ownerId,groupId)
	os.chown(rangerBaseDirName,ownerId,groupId)

	initializeInitD()

	tagsyncKSPath = mergeProps['ranger.tagsync.keystore.filename']

	if ('ranger.tagsync.dest.ranger.username' not in mergeProps):
		mergeProps['ranger.tagsync.dest.ranger.username'] = 'rangertagsync'

	if (tagsyncKSPath != ''):
		tagadminPasswd = 'rangertagsync'
		tagadminAlias = 'tagadmin.user.password'
		updatePropertyInJCKSFile(tagsyncKSPath,tagadminAlias,tagadminPasswd)
		os.chown(tagsyncKSPath,ownerId,groupId)

	tagsyncAtlasKSPath = mergeProps['ranger.tagsync.source.atlasrest.keystore.filename']

	if ('ranger.tagsync.source.atlasrest.username' not in mergeProps):
		mergeProps['ranger.tagsync.source.atlasrest.username'] = 'admin'

	if (tagsyncAtlasKSPath != ''):
		if ('ranger.tagsync.source.atlasrest.password' not in mergeProps):
			atlasPasswd = 'admin'
		else:
			atlasPasswd = mergeProps['ranger.tagsync.source.atlasrest.password']
			mergeProps.pop('ranger.tagsync.source.atlasrest.password')

		atlasAlias = 'atlas.user.password'
		updatePropertyInJCKSFile(tagsyncAtlasKSPath,atlasAlias,atlasPasswd)
		os.chown(tagsyncAtlasKSPath,ownerId,groupId)

	writeXMLUsingProperties(fn, mergeProps, outfn)

	fixPermList = [ ".", tagsyncBaseDirName, confFolderName ]

	for dir in fixPermList:
		for root, dirs, files in os.walk(dir):
			os.chown(root, ownerId, groupId)
			os.chmod(root,0o755)
			for obj in dirs:
				dn = join(root,obj)
				os.chown(dn, ownerId, groupId)
				os.chmod(dn, 0o755)
			for obj in files:
				fn = join(root,obj)
				os.chown(fn, ownerId, groupId)
				os.chmod(fn, 0o755)

	write_env_files("RANGER_TAGSYNC_HADOOP_CONF_DIR", hadoop_conf, ENV_HADOOP_CONF_FILE)
	write_env_files("TAGSYNC_PID_DIR_PATH", pid_dir_path, ENV_PID_FILE);
	write_env_files("TAGSYNC_CONF_DIR", os.path.join(tagsyncBaseDirFullName,confBaseDirName), ENV_CONF_FILE)
	os.chown(os.path.join(confBaseDirName, ENV_HADOOP_CONF_FILE),ownerId,groupId)
	os.chmod(os.path.join(confBaseDirName, ENV_HADOOP_CONF_FILE),0o755)
	os.chown(os.path.join(confBaseDirName, ENV_PID_FILE),ownerId,groupId)
	os.chmod(os.path.join(confBaseDirName, ENV_PID_FILE),0o755)
	os.chown(os.path.join(confBaseDirName, ENV_CONF_FILE),ownerId,groupId)
	os.chmod(os.path.join(confBaseDirName, ENV_CONF_FILE),0o755)

	f = open(os.path.join(confBaseDirName, ENV_PID_FILE), "a+")
	f.write("\nexport {0}={1}".format("UNIX_TAGSYNC_USER",unix_user))
	f.close()

	hadoop_conf_full_path = os.path.join(hadoop_conf, hadoopConfFileName)
	tagsync_conf_full_path = os.path.join(tagsyncBaseDirFullName,confBaseDirName,hadoopConfFileName)

	if not isfile(hadoop_conf_full_path):
		print("WARN: core-site.xml file not found in provided hadoop conf path...")
		f = open(tagsync_conf_full_path, "w")
		f.write("<configuration></configuration>")
		f.close()
		os.chown(tagsync_conf_full_path,ownerId,groupId)
		os.chmod(tagsync_conf_full_path,0o750)
	else:
		if os.path.islink(tagsync_conf_full_path):
			os.remove(tagsync_conf_full_path)

	if isfile(hadoop_conf_full_path) and not isfile(tagsync_conf_full_path):
		os.symlink(hadoop_conf_full_path, tagsync_conf_full_path)

	rangerTagsync_password = globalDict['rangerTagsync_password']
	rangerTagsync_name ='rangerTagsync'
	endPoint='RANGER'
	cmd = 'python updatetagadminpassword.py %s %s %s'  %(endPoint, rangerTagsync_name, rangerTagsync_password)
	if rangerTagsync_password != "" :
		output = os.system(cmd)
		if (output == 0):
			print("[I] Successfully updated password of " + rangerTagsync_name +" user")
		else:
			print("[ERROR] Unable to change password of " + rangerTagsync_name +" user.")
	print("\nINFO: Completed ranger-tagsync installation.....\n")

main()
